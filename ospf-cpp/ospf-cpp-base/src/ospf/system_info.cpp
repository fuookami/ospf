#include <ospf/system_info.hpp>
#include <ospf/literal_constant.hpp>
#include <algorithm>
#include <memory>
#include <optional>
#include <cstdint>
#include <bit>

#ifdef _WIN32
#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifdef BOOST_MSVC
#pragma comment(lib, "Iphlpapi.lib")
#endif
#endif

#ifdef OSPF_SUPPORT_CPU_INFO
#ifdef BOOST_MSVC
#if BOOST_MSVC > 1400
#include <Intrin.h>
#include <windows.h>
#endif
#elif defined(BOOST_GCC) || defined(BOOST_CLANG)
#include <cpuid.h>
#include <unistd.h>
#endif
#endif

#ifdef _WIN32
#include <WinSock2.h>
#include <Iphlpapi.h>
#else
#include <sys/types.h>
#include <sys/socket.h>
#include <sys/ioctl.h>
#include <net/if.h>
#include <arpa/inet.h>
#include <ifaddrs.h>
#include <cstring>
#endif

namespace ospf::detail
{
#ifdef OSPF_SUPPORT_CPU_INFO
    void get_cpu_id_ex(u32 cpu_info[4], i32 info_type, i32 ecx_value) noexcept
    {
#ifndef OSPF_OS_UNKNOWN
#ifdef BOOST_MSVC
#if defined(_WIN64) || _MSC_VER >= 1600
        __cpuidex(reinterpret_cast<i32*>(cpu_info), info_type, ecx_value);
#else
        if (cpu_info != 0)
        {
            _asm {
                mov edi, cpu_info;
                mov eax, info_type;
                mov ecx, ecx_value;
                cpuid;
                mov[edi], eax;
                mov[edi + 4], ebx;
                mov[edi + 8], ecx;
                mov[edi + 12], edx;
            }
        }

#endif
#elif defined(BOOST_GCC) || defined(BOOST_CLANG)
        __cpuid_count(info_type, ecx_value, cpu_info[0], cpu_info[1], cpu_info[2], cpu_info[3]);
#endif
#endif
    }

    void get_cpu_id(u32 cpu_info[4], i32 info_type) noexcept
    {
#ifndef OSPF_OS_UNKNOWN
#ifdef BOOST_MSVC
#if _MSC_VER >= 1400
        __cpuid(reinterpret_cast<i32*>(cpu_info), info_type);
#else
        get_cpu_id_ex(cpu_info, info_type, 0);
#endif
#elif defined(BOOST_GCC) || defined(BOOST_CLANG)
        __cpuid(info_type, cpu_info[0], cpu_info[1], cpu_info[2], cpu_info[3]);
#endif
#endif
    }

    const CPUInfo::ID get_cpu_id(const u32 cpu_info[4]) noexcept
    {
        CPUInfo::ID cpu_id;
        cpu_id[0] = static_cast<ubyte>(0xFF & (cpu_info[3] >> 24));
        cpu_id[1] = static_cast<ubyte>(0xFF & (cpu_info[3] >> 16));
        cpu_id[2] = static_cast<ubyte>(0xFF & (cpu_info[3] >> 8));
        cpu_id[3] = static_cast<ubyte>(0xFF & (cpu_info[3]));

        cpu_id[4] = static_cast<ubyte>(0xFF & (cpu_info[0] >> 24));
        cpu_id[5] = static_cast<ubyte>(0xFF & (cpu_info[0] >> 16));
        cpu_id[6] = static_cast<ubyte>(0xFF & (cpu_info[0] >> 8));
        cpu_id[7] = static_cast<ubyte>(0xFF & (cpu_info[0]));
        return cpu_id;
    }

    const usize get_cpu_core_number(void) noexcept
    {
        static usize num = 0_uz;
        if (num != 0_uz)
        {
            return num;
        }
#ifdef BOOST_MSVC
        SYSTEM_INFO si;
        GetSystemInfo(&si);
        num = si.dwNumberOfProcessors;
#elif defined(BOOST_GCC) || defined(BOOST_CLANG)
        num = sysconf(_SC_NPROCESSORS_CONF);
#endif
        return num;
    }

    const CPUInfo::ID get_cpu_id(void) noexcept
    {
#ifndef OSPF_OS_UNKNOWN
        u32 cpu_info[4];
        get_cpu_id(cpu_info, 1);
        return get_cpu_id(cpu_info);
#else
        return { static_cast<ubyte>(0) };
#endif
    }

    const CPUInfo get_cpu_info(void) noexcept
    {
        return { get_cpu_core_number(), get_cpu_id() };
    }
#else
    const CPUInfo get_cpu_info(void) noexcept
    {
        return { 1_uz, { static_cast<ubyte>(0xFF) } };
    }
#endif

#ifdef _WIN32
    std::vector<MACInfo> get_mac_info(void) noexcept
    {
        auto ip_adapter_info = std::make_unique<IP_ADAPTER_INFO>();
        auto size = sizeof(IP_ADAPTER_INFO);
        int ret_code = GetAdaptersInfo(ip_adapter_info.get(), reinterpret_cast<PULONG>(&size));
        int net_card_amount = 0;
        int ip_num_per_net_card = 0;
        if (ret_code == ERROR_BUFFER_OVERFLOW)
        {
            ip_adapter_info.reset(reinterpret_cast<PIP_ADAPTER_INFO>(new BYTE[size]));
            ret_code = GetAdaptersInfo(ip_adapter_info.get(), reinterpret_cast<PULONG>(&size));
        }

        std::vector<MACInfo> ret;
        auto ptr = ip_adapter_info.get();
        while (ptr != nullptr)
        {
            MACInfo mac{ { static_cast<ubyte>(0) } };
            std::copy(reinterpret_cast<const ubyte*>(ptr->Address), reinterpret_cast<const ubyte*>(ptr->Address) + MACInfo::address_length, mac.address.begin());
            ret.push_back(std::move(mac));
            ptr = ptr->Next;
        }
        return ret;
    }
#else
    std::optional<MACInfo> get_mac(void) noexcept
    {
        struct ifreq if_req;
        int sock;
        if ((sock = socket(AF_INET, SOCK_STREAM, 0)) < 0
            || ioctl(sock, SIOCGIFHWADDR, &if_req) < 0
            )
        {
            return std::nullopt;
        }

        MACInfo mac{ { static_cast<ubyte>(0) } };
        std::copy(reinterpret_cast<const ubyte*>(if_req.ifr_hwaddr.sa_data), reinterpret_cast<const ubyte*>(if_req.ifr_hwaddr.sa_data) + MACInfo::address_length, mac.address.begin());
        return std::move(mac);
    }

    std::vector<MACInfo> get_mac_info(void) noexcept
    {
        struct ifaddrs* if_addr = nullptr;
        getifaddrs(&if_addr);

        std::vector<MACInfo> ret;
        auto ptr = if_addr;
        while (ptr != nullptr)
        {
            if (ptr->ifa_addr->sa_family == AF_INET)
            {
                auto addr = &(reinterpret_cast<struct sockaddr_in*>(ptr->ifa_addr)->sin_addr);
                char ip[INET_ADDRSTRLEN];
                inet_ntop(AF_INET, addr, ip, INET_ADDRSTRLEN);
                if (strcmp(ip, "127.0.0.1") != 0)
                {
                    auto mac = get_mac();
                    if (mac.has_value())
                    {
                        ret.push_back(std::move(mac.value()));
                    }
                }
            }
            else if (ptr->ifa_addr->sa_family == AF_INET6)
            {
                auto addr = &(reinterpret_cast<struct sockaddr_in*>(ptr->ifa_addr)->sin_addr);
                char ip[INET_ADDRSTRLEN];
                inet_ntop(AF_INET6, addr, ip, INET6_ADDRSTRLEN);
                if (strcmp(ip, "::") != 0)
                {
                    auto mac = get_mac();
                    if (mac.has_value())
                    {
                        ret.push_back(std::move(mac.value()));
                    }
                }
            }
            ptr = ptr->ifa_next;
        }
        freeifaddrs(if_addr);
        if_addr = nullptr;
        return ret;
    }
#endif

    const Endian get_local_endian(void) noexcept
    {
        return std::endian::native == std::endian::big
            ? Endian::Big
            : Endian::Little;
    }
};

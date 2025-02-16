#pragma once

#include <ospf/ospf_base_api.hpp>
#include <ospf/basic_definition.hpp>
#include <magic_enum.hpp>

#ifndef OSPF_PLATFORM
#if defined(_M_X64) || defined(__x86_64__) || defined(__amd64)
#define OSPF_PLATFORM X64
#define OSPF_PLATFORM_X64
#elif defined(_M_ARM64) || defined(__aarch64__)
#define OSPF_PLATFORM A64
#define OSPF_PLATFORM_A64
#elif defined(_M_IX86) || defined(__i386__)
#define OSPF_PLATFORM X86
#define OSPF_PLATFORM_X86
#elif defined(_M_ARM) || defined(__arm__)
#define OSPF_PLATFORM A32
#define OSPF_PLATFORM_A32
#elif defined(__IA64__)
#define OSPF_PLATFORM IA64
#define OSPF_PLATFORM_IA64
#else
#define OSPF_PLATFORM Unknown
#pragma message(OSPF_COMPILER_PREFIX OSPF_COMPILER_MESSAGE(OSPF_UNKNOWN_HARDWARE_PLATFORM))
#endif
#endif

#ifndef OSPF_BITS
#if defined(_WIN64) || defined(__x86_64__) || defined(__aarch64__)
#define OSPF_BITS 64
#elif defined(_WIN32) || defined(__i386__) || defined(__arm__)
#define OSPF_BITS 32
#else
#define OSPF_BITS 64
#endif
#endif

#ifndef OSPF_OS
#if (defined(linux) || defined(__linux) || defined(__linux__) || defined(__GNU__) || defined(__GLIBC__) || defined(__amd64)) && !defined(_CRAYC)
#define OSPF_OS Linux
#define OSPF_OS_LINUX
#elif defined(_WIN32) || defined(__WIN32__) || defined(WIN32)
#define OSPF_OS Windows
#define OSPF_OS_WINDOWS
#elif defined(macintosh) || defined(__APPLE__) || defined(__APPLE_CC__) || defined(__DARWIN__) || defined(__MACH__)
#define OSPF_OS MacOS
#define OSPF_OS_MACOS
#else
#pragma message(OSPF_UNKNOWN_OS)
#define OSPF_OS Unknown
#define OSPF_OS_UNKNOWN
#endif
#endif

#ifndef OSPF_SUPPORT_CPU_INFO
#ifdef BOOST_MSVC
#if BOOST_MSVC > 1400
#define OSPF_SUPPORT_CPU_INFO
#endif
#elif defined(BOOST_GCC) || defined(BOOST_CLANG)
#define OSPF_SUPPORT_CPU_INFO
#endif
#endif

namespace ospf
{
    enum class HardwarePlatform : u8
    {
        X64,
        X86,
        A64,
        A32,
        IA64,
        Unknown
    };

    enum class OperationSystem : u8
    {
        Windows,
        Linux,
        MacOS,
        Unknown
    };

    struct CPUInfo
    {
        static constexpr const usize id_length = 8ULL;
        using ID = std::array<ubyte, id_length>;

        usize core_number;
        ID id;
    };

    struct MACInfo
    {
        static constexpr const usize address_length = 6ULL;
        using Address = std::array<ubyte, address_length>;

        Address address;
    };

    enum class Endian : u8
    {
        Big,
        Little
    };

    static constexpr const HardwarePlatform local_platform = HardwarePlatform::OSPF_PLATFORM;
    static constexpr const u64 address_length = sizeof(void*);
    static constexpr const u64 program_bits = address_length * 8;
    static constexpr const OperationSystem local_os = OperationSystem::OSPF_OS;

    namespace detail
    {
        OSPF_BASE_API const CPUInfo get_cpu_info(void) noexcept;
        OSPF_BASE_API std::vector<MACInfo> get_mac_info(void) noexcept;
        OSPF_BASE_API const Endian get_local_endian(void) noexcept;
    };

    static const CPUInfo local_cpu_info = detail::get_cpu_info();
    static const std::vector<MACInfo> local_mac_info = detail::get_mac_info();
    static const Endian local_endian = detail::get_local_endian();
};

namespace magic_enum::customize
{
    template <>
    constexpr customize_t enum_name<ospf::HardwarePlatform>(const ospf::HardwarePlatform value) noexcept
    {
        switch (value) {
        case ospf::HardwarePlatform::X64:
            return "x64";
        case ospf::HardwarePlatform::X86:
            return "x86";
        case ospf::HardwarePlatform::A64:
            return "a64";
        case ospf::HardwarePlatform::A32:
            return "a32";
        case ospf::HardwarePlatform::IA64:
            return "ia64";
        case ospf::HardwarePlatform::Unknown:
            return "unknown";
        default:
            return default_tag;
        }
    }

    template<>
    constexpr customize_t enum_name<ospf::OperationSystem>(const ospf::OperationSystem value) noexcept
    {
        switch (value) {
        case ospf::OperationSystem::Windows:
            return "Windows";
        case ospf::OperationSystem::Linux:
            return "Linux";
        case ospf::OperationSystem::MacOS:
            return "Mac OS";
        case ospf::OperationSystem::Unknown:
            return "unknown";
        default:
            return default_tag;
        }
    }
};

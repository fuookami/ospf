#include <ospf/log/multi_thread_impl.hpp>

namespace ospf::log::log_detail
{
    template class MultiThreadImpl<char>;
    template class MultiThreadImpl<wchar>;
};

#include <ospf/log/record.hpp>

namespace ospf::log
{
    template class LogRecord<char>;
    template class LogRecord<wchar>;
};

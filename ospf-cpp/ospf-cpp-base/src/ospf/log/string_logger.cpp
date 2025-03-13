#include <ospf/log/string.hpp>

namespace ospf::log
{
    template class DynStringLogger<on, char>;
    template class DynStringLogger<off, char>;
    template class DynStringLogger<on, wchar>;
    template class DynStringLogger<off, wchar>;

    template class StringLogger<LogLevel::Trace, on, char>;
    template class StringLogger<LogLevel::Trace, off, char>;
    template class StringLogger<LogLevel::Trace, on, wchar>;
    template class StringLogger<LogLevel::Trace, off, wchar>;

    template class StringLogger<LogLevel::Debug, on, char>;
    template class StringLogger<LogLevel::Debug, off, char>;
    template class StringLogger<LogLevel::Debug, on, wchar>;
    template class StringLogger<LogLevel::Debug, off, wchar>;

    template class StringLogger<LogLevel::Info, on, char>;
    template class StringLogger<LogLevel::Info, off, char>;
    template class StringLogger<LogLevel::Info, on, wchar>;
    template class StringLogger<LogLevel::Info, off, wchar>;

    template class StringLogger<LogLevel::Warn, on, char>;
    template class StringLogger<LogLevel::Warn, off, char>;
    template class StringLogger<LogLevel::Warn, on, wchar>;
    template class StringLogger<LogLevel::Warn, off, wchar>;

    template class StringLogger<LogLevel::Error, on, char>;
    template class StringLogger<LogLevel::Error, off, char>;
    template class StringLogger<LogLevel::Error, on, wchar>;
    template class StringLogger<LogLevel::Error, off, wchar>;

    template class StringLogger<LogLevel::Fatal, on, char>;
    template class StringLogger<LogLevel::Fatal, off, char>;
    template class StringLogger<LogLevel::Fatal, on, wchar>;
    template class StringLogger<LogLevel::Fatal, off, wchar>;
};

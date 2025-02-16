#include <ospf/log/file.hpp>

namespace ospf::log
{
    template class DynFileLogger<on, char>;
    template class DynFileLogger<off, char>;
    template class DynFileLogger<on, wchar>;
    template class DynFileLogger<off, wchar>;

    template class FileLogger<LogLevel::Trace, on, char>;
    template class FileLogger<LogLevel::Trace, off, char>;
    template class FileLogger<LogLevel::Trace, on, wchar>;
    template class FileLogger<LogLevel::Trace, off, wchar>;

    template class FileLogger<LogLevel::Debug, on, char>;
    template class FileLogger<LogLevel::Debug, off, char>;
    template class FileLogger<LogLevel::Debug, on, wchar>;
    template class FileLogger<LogLevel::Debug, off, wchar>;

    template class FileLogger<LogLevel::Info, on, char>;
    template class FileLogger<LogLevel::Info, off, char>;
    template class FileLogger<LogLevel::Info, on, wchar>;
    template class FileLogger<LogLevel::Info, off, wchar>;

    template class FileLogger<LogLevel::Warn, on, char>;
    template class FileLogger<LogLevel::Warn, off, char>;
    template class FileLogger<LogLevel::Warn, on, wchar>;
    template class FileLogger<LogLevel::Warn, off, wchar>;

    template class FileLogger<LogLevel::Error, on, char>;
    template class FileLogger<LogLevel::Error, off, char>;
    template class FileLogger<LogLevel::Error, on, wchar>;
    template class FileLogger<LogLevel::Error, off, wchar>;

    template class FileLogger<LogLevel::Fatal, on, char>;
    template class FileLogger<LogLevel::Fatal, off, char>;
    template class FileLogger<LogLevel::Fatal, on, wchar>;
    template class FileLogger<LogLevel::Fatal, off, wchar>;
};

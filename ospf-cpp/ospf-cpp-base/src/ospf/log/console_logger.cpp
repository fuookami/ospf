#include <ospf/log/console.hpp>

namespace ospf::log
{
    template class DynConsoleLogger<on, char>;
    template class DynConsoleLogger<off, char>;
    template class DynConsoleLogger<on, wchar>;
    template class DynConsoleLogger<off, wchar>;

    template class ConsoleLogger<LogLevel::Trace, on, char>;
    template class ConsoleLogger<LogLevel::Trace, off, char>;
    template class ConsoleLogger<LogLevel::Trace, on, wchar>;
    template class ConsoleLogger<LogLevel::Trace, off, wchar>;

    template class ConsoleLogger<LogLevel::Debug, on, char>;
    template class ConsoleLogger<LogLevel::Debug, off, char>;
    template class ConsoleLogger<LogLevel::Debug, on, wchar>;
    template class ConsoleLogger<LogLevel::Debug, off, wchar>;

    template class ConsoleLogger<LogLevel::Info, on, char>;
    template class ConsoleLogger<LogLevel::Info, off, char>;
    template class ConsoleLogger<LogLevel::Info, on, wchar>;
    template class ConsoleLogger<LogLevel::Info, off, wchar>;

    template class ConsoleLogger<LogLevel::Warn, on, char>;
    template class ConsoleLogger<LogLevel::Warn, off, char>;
    template class ConsoleLogger<LogLevel::Warn, on, wchar>;
    template class ConsoleLogger<LogLevel::Warn, off, wchar>;

    template class ConsoleLogger<LogLevel::Error, on, char>;
    template class ConsoleLogger<LogLevel::Error, off, char>;
    template class ConsoleLogger<LogLevel::Error, on, wchar>;
    template class ConsoleLogger<LogLevel::Error, off, wchar>;

    template class ConsoleLogger<LogLevel::Fatal, on, char>;
    template class ConsoleLogger<LogLevel::Fatal, off, char>;
    template class ConsoleLogger<LogLevel::Fatal, on, wchar>;
    template class ConsoleLogger<LogLevel::Fatal, off, wchar>;
};

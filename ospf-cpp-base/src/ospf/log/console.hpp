#pragma once

#include <ospf/config.hpp>
#include <ospf/log/interface.hpp>

namespace ospf
{
    inline namespace log
    {
        template<
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class DynConsoleLogger
            : public log_detail::DynLoggerImpl<mt, CharT, DynConsoleLogger<mt, CharT>>
        {
            using Impl = log_detail::DynLoggerImpl<mt, CharT, DynConsoleLogger<mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            DynConsoleLogger(const LogLevel lowest_level, std::basic_ostream<CharT>& os)
                : Impl(lowest_level), _os(os), _writer(RecordType::default_writer(os)) {}

            template<typename = void>
                requires (mt == off)
            DynConsoleLogger(const LogLevel lowest_level, std::basic_ostream<CharT>& os, const typename RecordType::WriterGenerator& writer_generator)
                : Impl(lowest_level), _os(os), _writer(writer_generator(os)) {}

            template<typename = void>
                requires (mt == on)
            DynConsoleLogger(const LogLevel lowest_level, std::basic_ostream<CharT>& os, const bool with_buffer = true)
                : Impl(lowest_level, with_buffer), _os(os), _writer(RecordType::default_writer(os)) {}

            template<typename = void>
                requires (mt == on)
            DynConsoleLogger(const LogLevel lowest_level, std::basic_ostream<CharT>& os, const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(lowest_level, with_buffer), _os(os), _writer(writer_generator(os)) {}

        public:
            DynConsoleLogger(const DynConsoleLogger& ano) = delete;
            DynConsoleLogger(DynConsoleLogger&& ano) noexcept = default;
            DynConsoleLogger& operator=(const DynConsoleLogger& rhs) = delete;
            DynConsoleLogger& operator=(DynConsoleLogger&& rhs) = delete;
            ~DynConsoleLogger(void) = default;

        public:
            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(*_os);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                (*_os) << std::move(message);
            }

        private:
            Ref<std::basic_ostream<CharT>> _os;
            RecordType::Writer _writer;
        };

        template<
            LogLevel lowest_level = default_log_level,
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class ConsoleLogger
            : public log_detail::LoggerImpl<lowest_level, mt, CharT, ConsoleLogger<lowest_level, mt, CharT>>
        {
            using Impl = log_detail::LoggerImpl<lowest_level, mt, CharT, ConsoleLogger<lowest_level, mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            ConsoleLogger(std::basic_ostream<CharT>& os)
                : _os(os), _writer(RecordType::default_writer(os)) {}

            template<typename = void>
                requires (mt == off)
            ConsoleLogger(std::basic_ostream<CharT>& os, const typename RecordType::WriterGenerator& writer_generator)
                : _os(os), _writer(writer_generator(os)) {}

            template<typename = void>
                requires (mt == on)
            ConsoleLogger(std::basic_ostream<CharT>& os, const bool with_buffer = true)
                : Impl(with_buffer), _os(os), _writer(RecordType::default_writer(os)) {}

            template<typename = void>
                requires (mt == on)
            ConsoleLogger(std::basic_ostream<CharT>& os, const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(with_buffer), _os(os), _writer(writer_generator(os)) {}

        public:
            ConsoleLogger(const ConsoleLogger& ano) = delete;
            ConsoleLogger(ConsoleLogger&& ano) noexcept = default;
            ConsoleLogger& operator=(const ConsoleLogger& rhs) = delete;
            ConsoleLogger& operator=(ConsoleLogger&& rhs) = delete;
            ~ConsoleLogger(void) = default;

        public:
            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(*_os);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                (*_os) << std::move(message);
            }

        private:
            Ref<std::basic_ostream<CharT>> _os;
            RecordType::Writer _writer;
        };

        extern template class DynConsoleLogger<on, char>;
        extern template class DynConsoleLogger<off, char>;
        extern template class DynConsoleLogger<on, wchar>;
        extern template class DynConsoleLogger<off, wchar>;

        extern template class ConsoleLogger<LogLevel::Trace, on, char>;
        extern template class ConsoleLogger<LogLevel::Trace, off, char>;
        extern template class ConsoleLogger<LogLevel::Trace, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Trace, off, wchar>;

        extern template class ConsoleLogger<LogLevel::Debug, on, char>;
        extern template class ConsoleLogger<LogLevel::Debug, off, char>;
        extern template class ConsoleLogger<LogLevel::Debug, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Debug, off, wchar>;

        extern template class ConsoleLogger<LogLevel::Info, on, char>;
        extern template class ConsoleLogger<LogLevel::Info, off, char>;
        extern template class ConsoleLogger<LogLevel::Info, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Info, off, wchar>;

        extern template class ConsoleLogger<LogLevel::Warn, on, char>;
        extern template class ConsoleLogger<LogLevel::Warn, off, char>;
        extern template class ConsoleLogger<LogLevel::Warn, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Warn, off, wchar>;

        extern template class ConsoleLogger<LogLevel::Error, on, char>;
        extern template class ConsoleLogger<LogLevel::Error, off, char>;
        extern template class ConsoleLogger<LogLevel::Error, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Error, off, wchar>;

        extern template class ConsoleLogger<LogLevel::Fatal, on, char>;
        extern template class ConsoleLogger<LogLevel::Fatal, off, char>;
        extern template class ConsoleLogger<LogLevel::Fatal, on, wchar>;
        extern template class ConsoleLogger<LogLevel::Fatal, off, wchar>;
    };
};

#pragma once

#include <ospf/config.hpp>
#include <ospf/log/interface.hpp>
#include <sstream>

namespace ospf
{
    inline namespace log
    {
        template<
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class DynStringLogger
            : public log_detail::DynLoggerImpl<mt, CharT, DynStringLogger<mt, CharT>>
        {
            using Impl = log_detail::DynLoggerImpl<mt, CharT, DynStringLogger<mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            DynStringLogger(const LogLevel lowest_level)
                : Impl(lowest_level), _sout(), _writer(RecordType::default_writer(_sout)) {}

            template<typename = void>
                requires (mt == off)
            DynStringLogger(const LogLevel lowest_level, const typename RecordType::WriterGenerator& writer_generator)
                : Impl(lowest_level), _sout(), _writer(writer_generator(_sout)) {}

            template<typename = void>
                requires (mt == on)
            DynStringLogger(const LogLevel lowest_level, const bool with_buffer = true)
                : Impl(lowest_level, with_buffer), _sout(), _writer(RecordType::default_writer(_sout)) {}

            template<typename = void>
                requires (mt == on)
            DynStringLogger(const LogLevel lowest_level, const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(lowest_level, with_buffer), _sout(), _writer(writer_generator(_sout)) {}

        public:
            DynStringLogger(const DynStringLogger& ano) = delete;
            DynStringLogger(DynStringLogger&& ano) noexcept = default;
            DynStringLogger& operator=(const DynStringLogger& rhs) = delete;
            DynStringLogger& operator=(DynStringLogger&& rhs) = delete;
            ~DynStringLogger(void) = default;

        public:
            inline StringType str(void) noexcept
            {
                return _sout.str();
            }

            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(_sout);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                _sout << std::move(message);
            }

        private:
            std::basic_ostringstream<CharT> _sout;
            RecordType::Writer _writer;
        };

        template<
            LogLevel lowest_level = default_log_level,
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class StringLogger
            : public log_detail::LoggerImpl<lowest_level, mt, CharT, StringLogger<lowest_level, mt, CharT>>
        {
            using Impl = log_detail::LoggerImpl<lowest_level, mt, CharT, StringLogger<lowest_level, mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            StringLogger(void)
                : _sout(), _writer(RecordType::default_writer(_sout)) {}

            template<typename = void>
                requires (mt == off)
            StringLogger(const typename RecordType::WriterGenerator& writer_generator)
                : _sout(), _writer(writer_generator(_sout)) {}

            template<typename = void>
                requires (mt == on)
            StringLogger(const bool with_buffer = true)
                : Impl(with_buffer), _sout(), _writer(RecordType::default_writer(_sout)) {}

            template<typename = void>
                requires (mt == on)
            StringLogger(const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(with_buffer), _sout(), _writer(writer_generator(_sout)) {}

        public:
            StringLogger(const StringLogger& ano) = delete;
            StringLogger(StringLogger&& ano) noexcept = default;
            StringLogger& operator=(const StringLogger& rhs) = delete;
            StringLogger& operator=(StringLogger&& rhs) = delete;
            ~StringLogger(void) = default;

        public:
            inline StringType str(void) noexcept
            {
                return _sout.str();
            }

            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(_sout);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                _sout << std::move(message);
            }

        private:
            std::basic_ostringstream<CharT> _sout;
            RecordType::Writer _writer;
        };

        extern template class DynStringLogger<on, char>;
        extern template class DynStringLogger<off, char>;
        extern template class DynStringLogger<on, wchar>;
        extern template class DynStringLogger<off, wchar>;

        extern template class StringLogger<LogLevel::Trace, on, char>;
        extern template class StringLogger<LogLevel::Trace, off, char>;
        extern template class StringLogger<LogLevel::Trace, on, wchar>;
        extern template class StringLogger<LogLevel::Trace, off, wchar>;

        extern template class StringLogger<LogLevel::Debug, on, char>;
        extern template class StringLogger<LogLevel::Debug, off, char>;
        extern template class StringLogger<LogLevel::Debug, on, wchar>;
        extern template class StringLogger<LogLevel::Debug, off, wchar>;

        extern template class StringLogger<LogLevel::Info, on, char>;
        extern template class StringLogger<LogLevel::Info, off, char>;
        extern template class StringLogger<LogLevel::Info, on, wchar>;
        extern template class StringLogger<LogLevel::Info, off, wchar>;

        extern template class StringLogger<LogLevel::Warn, on, char>;
        extern template class StringLogger<LogLevel::Warn, off, char>;
        extern template class StringLogger<LogLevel::Warn, on, wchar>;
        extern template class StringLogger<LogLevel::Warn, off, wchar>;

        extern template class StringLogger<LogLevel::Error, on, char>;
        extern template class StringLogger<LogLevel::Error, off, char>;
        extern template class StringLogger<LogLevel::Error, on, wchar>;
        extern template class StringLogger<LogLevel::Error, off, wchar>;

        extern template class StringLogger<LogLevel::Fatal, on, char>;
        extern template class StringLogger<LogLevel::Fatal, off, char>;
        extern template class StringLogger<LogLevel::Fatal, on, wchar>;
        extern template class StringLogger<LogLevel::Fatal, off, wchar>;
    };
};

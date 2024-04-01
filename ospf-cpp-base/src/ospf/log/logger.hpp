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
        class DynLogger
            : public log_detail::DynLoggerImpl<mt, CharT, DynLogger<mt, CharT>>
        {
            using Impl = log_detail::DynLoggerImpl<mt, CharT, DynLogger<mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            DynLogger(RecordType::Writer writer)
                : _writer(std::move(writer)) {}
            DynLogger(const DynLogger& ano) = delete;
            DynLogger(DynLogger&& ano) noexcept = default;
            DynLogger& operator=(const DynLogger& rhs) = delete;
            DynLogger& operator=(DynLogger&& rhs) = delete;
            ~DynLogger(void) = default;

        public:
            inline void set_writer(RecordType::Writer writer) noexcept
            {
                _writer = std::move(writer);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                this->log(LogLevel::Other, std::move(message));
            }

        private:
            RecordType::Writer _writer;
        };

        template<
            LogLevel lowest_level = default_log_level,
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class Logger
            : public log_detail::LoggerImpl<lowest_level, mt, CharT, Logger<lowest_level, mt, CharT>>
        {
            using Impl = log_detail::LoggerImpl<lowest_level, mt, CharT, Logger<mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            Logger(RecordType::Writer writer)
                : _writer(std::move(writer)) {}
            Logger(const Logger& ano) = delete;
            Logger(Logger&& ano) noexcept = default;
            Logger& operator=(const Logger& rhs) = delete;
            Logger& operator=(Logger&& rhs) = delete;
            ~Logger(void) = default;

        public:
            inline void set_writer(RecordType::Writer writer) noexcept
            {
                _writer = std::move(writer);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                this->log(LogLevel::Other, std::move(message));
            }

        private:
            RecordType::Writer _writer;
        };
    };
};

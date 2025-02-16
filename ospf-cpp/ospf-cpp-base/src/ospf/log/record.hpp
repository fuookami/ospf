#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/base.hpp>
#include <ospf/concepts/enum.hpp>
#include <ospf/log/level.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/string/format.hpp>
#include <chrono>
#include <functional>
#include <ostream>

namespace ospf
{
    inline namespace log
    {
        template<CharType CharT = char>
        class LogRecord
        {
        public:
            using DateTimeType = std::chrono::system_clock::time_point;
            using StringType = std::basic_string<CharT>;
            using StringViewType = std::basic_string_view<CharT>;
            using OutputStreamType = std::basic_ostream<CharT>;
            using Writer = std::function<void(const DateTimeType time, const LogLevel level, const StringViewType message)>;
            using WriterGenerator = std::function<Writer(OutputStreamType&)>;

        public:
            static Writer default_writer(OutputStreamType& os) noexcept
            {
                return [&os](const DateTimeType time, const LogLevel level, const StringViewType message)
                {
                    if (level != LogLevel::Other)
                    {
                        if constexpr (DecaySameAs<CharT, char>)
                        {
                            os << std::format("[{}] {0:%F}T{0:%T%z}: {}", level, time, message) << '\n';
                        }
                        else if constexpr (DecaySameAs<CharT, wchar>)
                        {
                            os << std::format(L"[{}] {0:%F}T{0:%T%z}: {}", level, time, message) << '\n';
                        }
                        //else
                        //{
                        //    static_assert(false, "unsupported character type");
                        //}
                    }
                    else
                    {
                        os << message << '\n';
                    }
                };
            }

        public:
            LogRecord(const LogLevel level, StringType message, const Writer& writer)
                : _time(std::chrono::system_clock::now()), _level(level), _message(std::move(message)), _writer(writer) {}
            LogRecord(const LogRecord& ano) = delete;
            LogRecord(LogRecord&& ano) = default;
            LogRecord& operator=(const LogRecord& rhs) = delete;
            LogRecord& operator=(LogRecord&& rhs) = delete;
            ~LogRecord(void) = default;

        public:
            inline const DateTimeType time(void) const noexcept
            {
                return _time;
            }

            inline const LogLevel level(void) const noexcept
            {
                return _level;
            }

            inline const StringViewType message(void) const noexcept
            {
                return _message;
            }

            inline const Writer& writer(void) const noexcept
            {
                return *_writer;
            }

        public:
            inline void write(void) const noexcept
            {
                (*_writer)(_time, _level, _message);
            }

        private:
            DateTimeType _time;
            LogLevel _level;
            StringType _message;
            Ref<Writer> _writer;
        };

        extern template class LogRecord<char>;
        extern template class LogRecord<wchar>;
    };
};

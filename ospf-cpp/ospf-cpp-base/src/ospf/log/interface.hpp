#pragma once

#include <ospf/error.hpp>
#include <ospf/log/multi_thread_impl.hpp>
#include <ospf/log/record.hpp>
#include <ospf/meta_programming/crtp.hpp>
#include <ospf/meta_programming/named_flag.hpp>
#include <ospf/meta_programming/variable_type_list.hpp>
#include <ospf/type_family.hpp>

namespace ospf
{
    inline namespace log
    {
        OSPF_NAMED_FLAG(LoggerMultiThread);

        template<CharType CharT = char>
        class DynLoggerInterface
        {
        public:
            using CharType = CharT;
            using RecordType = LogRecord<CharT>;
            using StringType = std::basic_string<CharT>;
            using StringViewType = std::basic_string_view<CharT>;

        public:
            DynLoggerInterface(const LogLevel lowest_level)
                : _lowest_level(lowest_level) {}
            DynLoggerInterface(const DynLoggerInterface& ano) = delete;
            DynLoggerInterface(DynLoggerInterface&& ano) noexcept = default;
            DynLoggerInterface& operator=(const DynLoggerInterface& rhs) = delete;
            DynLoggerInterface& operator=(DynLoggerInterface&& rhs) = delete;
            ~DynLoggerInterface(void) noexcept = default;

        public:
            inline const LogLevel lowest_level(void) const noexcept
            {
                return _lowest_level;
            }

            inline void set_lowest_level(const LogLevel new_level) noexcept
            {
                _lowest_level = new_level;
            }

        public:
            inline void log(void) noexcept
            {
                write(StringType{});
            }

            inline void log(StringType message) noexcept
            {
                write(std::move(message));
            }

        public:
            inline void log(const LogLevel level, StringType message) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(recored(level, std::move(message)));
                }
            }

            inline void log(const LogLevel level, const StringViewType message) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(recored(level, StringType{ message }));
                }
            }

            template<EnumType C>
            inline void log(Error<C, CharT> error) noexcept
            {
                if (LogLevel::Error >= _lowest_level)
                {
                    if constexpr (DecaySameAs<CharT, char>)
                    {
                        log(record(LogLevel::Error, std::format("{}, {}", error.code(), error.message())));
                    }
                    else if constexpr (DecaySameAs<CharT, wchar>)
                    {
                        log(record(LogLevel::Error, std::format(L"{}, {}", error.code(), error.message())));
                    }
                    //else
                    //{
                    //    static_assert(false, "unsupported character type");
                    //}
                }
            }

        public:
            template<typename... Args>
                requires (VariableTypeList<Args_...>::length != 0_uz)
            inline void log(const StringViewType fmt, Args&&... args) noexcept
            {
                write(std::vformat(fmt, make_format_args<CharT>(std::forward<Args>(args)...)));
            }

            template<typename... Args>
                requires (VariableTypeList<Args_...>::length != 0_uz)
            inline void log(const LogLevel level, const StringViewType fmt, Args&&... args) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(record(level, std::vformat(fmt, make_format_args<CharT>(std::forward<Args>(args)...))));
                }
            }

        protected:
            virtual void log(RecordType record) noexcept
            {
                assert(record.level() >= _lowest_level);
                record.write();
            }

        protected:
            virtual RecordType record(const LogLevel level, StringType message) const noexcept = 0;
            virtual void write(StringType message) noexcept = 0;

        private:
            LogLevel _lowest_level;
        };

        template<LogLevel _lowest_level, CharType CharT = char>
            requires (_lowest_level != LogLevel::Other)
        class LoggerInterface
        {
        public:
            using CharType = CharT;
            using RecordType = LogRecord<CharT>;
            using StringType = std::basic_string<CharT>;
            using StringViewType = std::basic_string_view<CharT>;

            static constexpr const LogLevel lowest_log_level = _lowest_level;

        public:
            LoggerInterface(void) = default;
            LoggerInterface(const LoggerInterface& ano) = delete;
            LoggerInterface(LoggerInterface&& ano) noexcept = default;
            LoggerInterface& operator=(const LoggerInterface& rhs) = delete;
            LoggerInterface& operator=(LoggerInterface&& rhs) = delete;
            ~LoggerInterface(void) noexcept = default;

        public:
            inline const LogLevel lowest_level(void) const noexcept
            {
                return _lowest_level;
            }

        public:
            inline void log(void) noexcept
            {
                write(StringType{});
            }

            inline void log(StringType message) noexcept
            {
                write(std::move(message));
            }

        public:
            inline void log(const LogLevel level, StringType message) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(recored(level, std::move(message)));
                }
            }

            inline void log(const LogLevel level, const StringViewType message) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(recored(level, StringType{ message }));
                }
            }

            template<LogLevel level>
            inline void log(StringType message) noexcept
            {
                if constexpr (level >= _lowest_level)
                {
                    log(recored(level, std::move(message)));
                }
            }

            template<LogLevel level>
            inline void log(const StringViewType message) noexcept
            {
                if constexpr (level >= _lowest_level)
                {
                    log(recored(level, StringType{ message }));
                }
            }

            template<EnumType C>
            inline void log(Error<C, CharT> error) noexcept
            {
                if constexpr (LogLevel::Error >= _lowest_level)
                {
                    static const auto fmt = boost::locale::conv::to_utf<CharT>("{}, {}", std::locale{});
                    log(record(LogLevel::Error, std::vformat(fmt, make_format_args<CharT>(error.code(), error.message()))));
                }
            }

        public:
            template<typename... Args>
                requires (VariableTypeList<Args_...>::length != 0_uz)
            inline void log(const StringViewType fmt, Args&&... args) noexcept
            {
                write(std::vformat(fmt, make_format_args<CharT>(std::forward<Args>(args)...)));
            }

            template<typename... Args>
                requires (VariableTypeList<Args_...>::length != 0_uz)
            inline void log(const LogLevel level, const StringViewType fmt, Args&&... args) noexcept
            {
                if (level >= _lowest_level)
                {
                    log(record(level, std::vformat(fmt, make_format_args<CharT>(std::forward<Args>(args)...))));
                }
            }

            template<LogLevel level, typename... Args>
                requires (VariableTypeList<Args_...>::length != 0_uz)
            inline void log(const StringViewType fmt, Args&&... args) noexcept
            {
                if constexpr (level >= _lowest_level)
                {
                    log(record(level, std::vformat(fmt, make_format_args<CharT>(std::forward<Args>(args)...))));
                }
            }

        protected:
            virtual void log(RecordType record) noexcept
            {
                assert(record.level() >= _lowest_level);
                record.write();
            }

        protected:
            virtual RecordType record(const LogLevel level, StringType message) const noexcept = 0;
            virtual void write(StringType message) noexcept = 0;
        };

        template<CharType CharT = char>
        using LoggerRefWrapper = std::variant<
            Ref<DynLoggerInterface<CharT>>,
            Ref<LoggerInterface<LogLevel::Trace, CharT>>,
            Ref<LoggerInterface<LogLevel::Debug, CharT>>,
            Ref<LoggerInterface<LogLevel::Info, CharT>>,
            Ref<LoggerInterface<LogLevel::Warn, CharT>>,
            Ref<LoggerInterface<LogLevel::Error, CharT>>,
            Ref<LoggerInterface<LogLevel::Fatal, CharT>>
        >;

        template<CharType CharT = char>
        using LoggerUniqueWrapper = std::variant<
            Unique<DynLoggerInterface<CharT>>,
            Unique<LoggerInterface<LogLevel::Trace, CharT>>,
            Unique<LoggerInterface<LogLevel::Debug, CharT>>,
            Unique<LoggerInterface<LogLevel::Info, CharT>>,
            Unique<LoggerInterface<LogLevel::Warn, CharT>>,
            Unique<LoggerInterface<LogLevel::Error, CharT>>,
            Unique<LoggerInterface<LogLevel::Fatal, CharT>>
        >;

        template<typename T, typename... Args>
            requires std::convertible_to<PtrType<T>, PtrType<DynLoggerInterface<typename T::CharType>>>
        inline LoggerUniqueWrapper<typename T::CharType> make_logger(Args&&... args) noexcept
        {
            auto new_logger = new T{ std::forward<Args>(args)... };
            return LoggerUniqueWrapper<typename T::CharType>{ Unique<DynLoggerInterface<typename T::CharType>>{ new_logger } };
        }

        template<typename T, typename... Args>
            requires std::convertible_to<PtrType<T>, PtrType<LoggerInterface<T::lowest_log_level, typename T::CharType>>>
        inline LoggerUniqueWrapper<typename T::CharType> make_logger(Args&&... args) noexcept
        {
            auto new_logger = new T{ std::forward<Args>(args)... };
            return LoggerUniqueWrapper<typename T::CharType>{ Unique<LoggerInterface<T::lowest_log_level, typename T::CharType>>{ new_logger } };
        }

        template<CharType CharT>
        inline LoggerRefWrapper<CharT> ref(const LoggerUniqueWrapper<CharT>& logger) noexcept
        {
            return std::visit([](const auto& logger)
                {
                    using LoggerType = OriginType<decltype(*logger)>;
                    return Ref<LoggerType>{ *logger };
                }, logger);
        }

        namespace log_detail
        {
            template<LoggerMultiThread mt, CharType CharT, typename Self>
            class DynLoggerImpl;

            template<CharType CharT, typename Self>
            class DynLoggerImpl<on, CharT, Self>
                : public DynLoggerInterface<CharT>
            {
                OSPF_CRTP_IMPL;
                using Interface = DynLoggerInterface<CharT>;

            public:
                using typename Interface::RecordType;
                using typename Interface::StringType;
                using typename Interface::StringViewType;

            public:
                DynLoggerImpl(const LogLevel lowest_level, const bool with_buffer)
                    : Interface(lowest_level), _impl(with_buffer) {}
                DynLoggerImpl(const DynLoggerImpl& ano) = delete;
                DynLoggerImpl(DynLoggerImpl&& ano) noexcept = default;
                DynLoggerImpl& operator=(const DynLoggerImpl& rhs) = delete;
                DynLoggerImpl& operator=(DynLoggerImpl&& rhs) = delete;
                ~DynLoggerImpl(void) = default;

            public:
                inline void join(void) noexcept
                {
                    _impl.join();
                }

                inline void flush(void) noexcept
                {
                    _impl.flush();
                }

            protected:
                void log(RecordType record) noexcept override
                {
                    assert(record.level() >= this->lowest_level());
                    _impl.add(std::move(record));
                }

                RecordType record(const LogLevel level, StringType message) const noexcept override
                {
                    assert(level >= this->lowest_level());
                    return Trait::make_record(self(), level, std::move(message));
                }

                void write(StringType message) noexcept override
                {
                    _impl.add(Trait::make_record(self(), LogLevel::Other, std::move(message)));
                }

            private:
                struct Trait : public Self
                {
                    inline static RecordType make_record(const Self& self, const LogLevel level, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(make_record);
                        return (self.*impl)(level, std::move(message));
                    }
                };

            private:
                MultiThreadImpl<CharT> _impl;
            };

            template<CharType CharT, typename Self>
            class DynLoggerImpl<off, CharT, Self>
                : public DynLoggerInterface<CharT>
            {
                OSPF_CRTP_IMPL;
                using Interface = DynLoggerInterface<CharT>;

            public:
                using typename Interface::RecordType;
                using typename Interface::StringType;
                using typename Interface::StringViewType;

            public:
                DynLoggerImpl(const LogLevel lowest_level)
                    : Interface(lowest_level) {}
                DynLoggerImpl(const DynLoggerImpl& ano) = delete;
                DynLoggerImpl(DynLoggerImpl&& ano) noexcept = default;
                DynLoggerImpl& operator=(const DynLoggerImpl& rhs) = delete;
                DynLoggerImpl& operator=(DynLoggerImpl&& rhs) = delete;
                ~DynLoggerImpl(void) = default;

            protected:
                RecordType record(const LogLevel level, StringType message) const noexcept override
                {
                    assert(level >= this->lowest_level());
                    return Trait::make_record(self(), level, std::move(message));
                }

                void write(StringType message) noexcept override
                {
                    Trait::write_message(self(), std::move(message));
                }

            private:
                struct Trait : public Self
                {
                    inline static RecordType make_record(const Self& self, const LogLevel level, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(make_record);
                        return (self.*impl)(level, std::move(message));
                    }

                    inline static void write_message(Self& self, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(write_message);
                        (self.*impl)(std::move(message));
                    }
                };
            };

            template<LogLevel _lowest_level, LoggerMultiThread mt, CharType CharT, typename Self>
            class LoggerImpl;

            template<LogLevel _lowest_level, CharType CharT, typename Self>
            class LoggerImpl<_lowest_level, on, CharT, Self>
                : public LoggerInterface<_lowest_level, CharT>
            {
                OSPF_CRTP_IMPL;
                using Interface = LoggerInterface<_lowest_level, CharT>;

            public:
                using typename Interface::RecordType;
                using typename Interface::StringType;
                using typename Interface::StringViewType;

            public:
                LoggerImpl(const bool with_buffer)
                    : _impl(with_buffer) {}
                LoggerImpl(const LoggerImpl& ano) = delete;
                LoggerImpl(LoggerImpl&& rhs) noexcept = default;
                LoggerImpl& operator=(const LoggerImpl& rhs) = delete;
                LoggerImpl& operator=(LoggerImpl&& rhs) = delete;
                ~LoggerImpl(void) = default;

            public:
                inline void join(void) noexcept
                {
                    _impl.join();
                }

                inline void flush(void) noexcept
                {
                    _impl.flush();
                }

            protected:
                void log(RecordType record) noexcept override
                {
                    assert(record.level() >= _lowest_level);
                    _impl.add(std::move(record));
                }

                RecordType record(const LogLevel level, StringType message) const noexcept override
                {
                    assert(level >= _lowest_level);
                    return Trait::make_record(self(), level, std::move(message));
                }

                void write(StringType message) noexcept override
                {
                    _impl.add(Trait::make_record(self(), LogLevel::Other, std::move(message)));
                }

            private:
                struct Trait : public Self
                {
                    inline static RecordType make_record(const Self& self, const LogLevel level, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(make_record);
                        return (self.*impl)(level, std::move(message));
                    }
                };

            private:
                MultiThreadImpl<CharT> _impl;
            };

            template<LogLevel _lowest_level, CharType CharT, typename Self>
            class LoggerImpl<_lowest_level, off, CharT, Self>
                : public LoggerInterface<_lowest_level, CharT>
            {
                OSPF_CRTP_IMPL;
                using Interface = LoggerInterface<_lowest_level, CharT>;

            public:
                using typename Interface::RecordType;
                using typename Interface::StringType;
                using typename Interface::StringViewType;

            public:
                LoggerImpl(void) = default;
                LoggerImpl(const LoggerImpl& ano) = delete;
                LoggerImpl(LoggerImpl&& rhs) noexcept = default;
                LoggerImpl& operator=(const LoggerImpl& rhs) = delete;
                LoggerImpl& operator=(LoggerImpl&& rhs) = delete;
                ~LoggerImpl(void) = default;

            protected:
                RecordType record(const LogLevel level, StringType message) const noexcept override
                {
                    assert(level >= _lowest_level);
                    return Trait::make_record(self(), level, std::move(message));
                }

                void write(StringType message) noexcept override
                {
                    Trait::write_message(self(), std::move(message));
                }

            private:
                struct Trait : public Self
                {
                    inline static RecordType make_record(const Self& self, const LogLevel level, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(make_record);
                        return (self.*impl)(level, std::move(message));
                    }

                    inline static void write_message(Self& self, StringType message) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(write_message);
                        (self.*impl)(std::move(message));
                    }
                };
            };
        };
    };
};

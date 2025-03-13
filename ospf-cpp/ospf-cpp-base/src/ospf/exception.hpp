#pragma once

#include <ospf/error.hpp>
#include <ospf/meta_programming/crtp.hpp>

namespace ospf
{
    namespace exception
    {
        template<ErrorType E, typename Self>
        class ExceptionImpl
            : public std::exception
        {
            OSPF_CRTP_IMPL

        public:
            using ErrType = OriginType<E>;

        public:
            constexpr ExceptionImpl(void) noexcept = default;
            constexpr ExceptionImpl(const ExceptionImpl& ano) = default;
            constexpr ExceptionImpl(ExceptionImpl&& ano) noexcept = default;
            constexpr ExceptionImpl& operator=(const ExceptionImpl& rhs) = default;
            constexpr ExceptionImpl& operator=(ExceptionImpl&& rhs) noexcept = default;
            constexpr ~ExceptionImpl(void) noexcept = default;

        public:
            inline CLRefType<ErrType> error(void) const & noexcept
            {
                return Trait::error(self());
            }

            inline RetType<ErrType> error(void) && noexcept
            {
                return Trait::error(static_cast<Self&&>(self()));
            }

            inline decltype(auto) code(void) const & noexcept
            {
                return error().code();
            }

            inline const std::string_view message(void) const & noexcept
            {
                return error().message();
            }

            inline CPtrType<char> what() const override
            {
                return message().data();
            }

        private:
            struct Trait : public Self
            {
                inline static CLRefType<ErrType> error(const Self& self) noexcept
                {
                    static const auto impl = &Self::OSPF_CRTP_FUNCTION(get_error);
                    return (self.*impl)();
                }

                inline static RetType<ErrType> error(Self&& self) noexcept
                {
                    static const auto impl = &Self::OSPF_CRTP_FUNCTION(get_moved_error);
                    return (self.*impl)();
                }
            };
        };
    };

    template<ErrorType E>
    class Exception
        : public exception::ExceptionImpl<OriginType<E>, Exception<E>>
    {
        using Impl = exception::ExceptionImpl<OriginType<E>, Exception<E>>;

    public:
        using typename Impl::ErrType;

    public:
        template<typename = void>
            requires WithDefault<ErrType>
        constexpr Exception(void)
            : Exception(DefaultValue<ErrType>::value()) {}

        constexpr Exception(ArgRRefType<ErrType> error)
            : _error(move<ErrType>(error)) {}

        template<typename... Args>
            requires std::is_constructible_v<ErrType, Args...>
        constexpr Exception(Args&&... args)
            : _error(std::forward<Args>(args)...) {}

    public:
        constexpr Exception(const Exception& ano) = default;
        constexpr Exception(Exception&& ano) noexcept = default;
        constexpr Exception& operator=(const Exception& rhs) = default;
        constexpr Exception& operator=(Exception&& rhs) noexcept = default;
        constexpr ~Exception(void) noexcept = default;

    OSPF_CRTP_PERMISSION:
        inline CLRefType<ErrType> OSPF_CRTP_FUNCTION(get_error)(void) const & noexcept
        {
            return _error;
        }

        inline RetType<ErrType> OSPF_CRTP_FUNCTION(get_moved_error)(void) && noexcept
        {
            return std::move(_error);
        }

    private:
        ErrType _error;
    };

    template<ExErrorType E>
    class ExException
        : public exception::ExceptionImpl<OriginType<E>, ExException<E>>
    {
        using Impl = exception::ExceptionImpl<OriginType<E>, ExException<E>>;

    public:
        using typename Impl::ErrType;

    public:
        template<typename = void>
            requires WithDefault<ErrType>
        constexpr ExException(void)
            : ExException(DefaultValue<ErrType>::value()) {}

        constexpr ExException(ArgRRefType<ErrType> error)
            : _error(move<ErrType>(error)) {}

        template<typename... Args>
            requires std::is_constructible_v<E, Args...>
        constexpr ExException(Args&&... args)
            : _error(std::forward<Args>(args)...) {}

    public:
        constexpr ExException(const ExException& ano) = default;
        constexpr ExException(ExException&& ano) noexcept = default;
        constexpr ExException& operator=(const ExException& rhs) = default;
        constexpr ExException& operator=(ExException&& rhs) noexcept = default;
        constexpr ~ExException(void) noexcept = default;

    public:
        inline decltype(auto) arg(void) const & noexcept
        {
            return _error.arg();
        }

        inline decltype(auto) arg(void) && noexcept
        {
            return static_cast<ErrType&&>(_error).arg();
        }

    OSPF_CRTP_PERMISSION:
        inline CLRefType<ErrType> OSPF_CRTP_FUNCTION(get_error)(void) const & noexcept
        {
            return _error;
        }

        inline RetType<ErrType> OSPF_CRTP_FUNCTION(get_moved_error)(void) && noexcept
        {
            return std::move(_error);
        }

    private:
        ErrType _error;
    };

    extern template class Exception<OSPFError>;

    using OSPFException = Exception<OSPFError>;

    template<typename T>
    using ExOSPFException = ExException<ExOSPFError<T>>;
};

namespace ospf::concepts
{
    template<ErrorType E>
        requires WithDefault<E>
    struct DefaultValue<Exception<E>>
    {
        inline static constexpr Exception<E> value(void) noexcept
        {
            return Exception<E>{};
        }
    };

    template<ExErrorType E>
        requires WithDefault<E>
    struct DefaultValue<Exception<E>>
    {
        inline static constexpr ExException<E> value(void) noexcept
        {
            return ExException<E>{};
        }
    };
};

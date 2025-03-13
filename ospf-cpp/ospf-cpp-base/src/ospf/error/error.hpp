#pragma once

#include <ospf/concepts.hpp>
#include <ospf/type_family.hpp>
#include <ospf/error/code.hpp>
#include <magic_enum.hpp>
#include <concepts>
#include <optional>
#include <string>

namespace ospf
{
    inline namespace error
    {
        template<typename T>
        concept ErrorType = requires(const T & error)
        {
            { error.code() } -> EnumType;
            { error.message() } -> StringViewType;
        };

        template<typename T>
        concept ExErrorType = ErrorType<T> && requires(const T& error)
        {
            { error.arg() } -> NotVoidType;
        };

        template<EnumType C, CharType CharT>
        class Error
        {
        public:
            using CodeType = OriginType<C>;
            using StringType = std::basic_string<CharT>;
            using StringViewType = std::basic_string_view<CharT>;

        public:
            template<typename = void>
                requires WithDefault<C>
            constexpr Error(void)
                : Error(DefaultValue<C>::value()) {}

            constexpr Error(CodeType code)
                : _code(code), _msg(to_string<CodeType, CharT>(code)) {}

            constexpr explicit Error(CodeType code, StringType msg)
                : _code(code), _msg(std::move(msg)) {}

        public:
            constexpr Error(const Error& ano) = default;
            constexpr Error(Error&& ano) noexcept = default;
            constexpr Error& operator=(const Error& rhs) = default;
            constexpr Error& operator=(Error&& rhs) noexcept = default;
            constexpr ~Error(void) noexcept = default;

        public:
            inline constexpr const CodeType code(void) const & noexcept
            {
                return _code;
            }

            inline constexpr const StringViewType message(void) const & noexcept
            {
                return _msg;
            }

            inline StringType message(void) && noexcept
            {
                return std::move(_msg);
            }

        private:
            CodeType _code;
            StringType _msg;
        };

        template<EnumType C, NotVoidType T, CharType CharT>
        class ExError
            : public Error<C, CharT>
        {
            using Base = Error<C, CharT>;

        public:
            using typename Base::CodeType;
            using typename Base::StringType;
            using typename Base::StringViewType;
            using ArgType = T;

        public:
            constexpr ExError(void)
                : Base(), _arg(std::nullopt) {}

            constexpr ExError(CodeType code)
                : Base(code), _arg(std::nullopt) {}

            constexpr explicit ExError(CodeType code, StringType msg)
                : Base(code, std::move(msg)), _arg(std::nullopt) {}

            constexpr explicit ExError(CodeType code, StringType msg, ArgRRefType<ArgType> arg)
                : Base(code, std::move(msg)), _arg(move<ArgType>(arg)) {}

            constexpr explicit ExError(Base error, ArgRRefType<ArgType> arg)
                : Base(std::move(error)), _arg(move<ArgType>(arg)) {}

        public:
            constexpr ExError(const ExError& ano) = default;
            constexpr ExError(ExError&& ano) noexcept = default;
            constexpr ExError& operator=(const ExError& rhs) = default;
            constexpr ExError& operator=(ExError&& rhs) noexcept = default;
            constexpr ~ExError(void) noexcept = default;

        public:
            inline constexpr const std::optional<ArgType>& arg(void) const & noexcept
            {
                return _arg;
            }

            inline std::optional<ArgType> arg(void) && noexcept
            {
                return std::move(_arg);
            }

        private:
            std::optional<ArgType> _arg;
        };

        extern template class Error<OSPFErrCode, char>;
        extern template class Error<OSPFErrCode, wchar>;
        //extern template class Error<OSPFErrCode, u8char>;
        //extern template class Error<OSPFErrCode, u16char>;
        //extern template class Error<OSPFErrCode, u32char>;
    };
};

namespace std
{
    template<ospf::EnumType C>
    struct formatter<ospf::Error<C, char>, char>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::Error<C, char>& error, FormatContext& fc) const
        {
            static const formatter<string_view, char> _formatter{};
            return _formatter.format(format("{}, msg: {}", error.code(), error.message()), fc);
        }
    };

    template<ospf::EnumType C>
    struct formatter<ospf::Error<C, ospf::wchar>, ospf::wchar>
        : public formatter<wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::Error<C, ospf::wchar>& error, FormatContext& fc) const
        {
            static const formatter<wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(format(L"{}, msg: {}", error.code(), error.message()), fc);
        }
    };

    template<ospf::EnumType C, ospf::NotVoidType T>
    struct formatter<ospf::ExError<C, T, char>, char>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::ExError<C, T, char>& error, FormatContext& fc) const
        {
            static const formatter<string_view, char> _formatter{};
            return _formatter.format(format("{}, msg: {}, arg: {}", error.code(), error.message(), error.arg()), fc);
        }
    };

    template<ospf::EnumType C, ospf::NotVoidType T>
    struct formatter<ospf::ExError<C, T, ospf::wchar>, ospf::wchar>
        : public formatter<wstring_view, ospf::wchar>
    {
        template<typename FormatContext>
        inline constexpr decltype(auto) format(const ospf::ExError<C, T, ospf::wchar>& error, FormatContext& fc) const
        {
            static const formatter<wstring_view, ospf::wchar> _formatter{};
            return _formatter.format(format(L"{}, msg: {}, arg: {}", error.code(), error.message(), error.arg()), fc);
        }
    };
};

namespace ospf::concepts
{
    template<EnumType C, CharType CharT>
        requires WithDefault<C>
    struct DefaultValue<Error<C, CharT>>
    {
        inline static constexpr Error<C, CharT> value(void) noexcept
        {
            return Error<C, CharT>{};
        }
    };

    template<EnumType C, NotVoidType T, CharType CharT>
        requires WithDefault<C>
    struct DefaultValue<ExError<C, T, CharT>>
    {
        inline static constexpr ExError<C, T, CharT> value(void) noexcept
        {
            return ExError<C, T, CharT>{};
        }
    };
};

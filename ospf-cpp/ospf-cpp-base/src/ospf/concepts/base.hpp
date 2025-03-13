#pragma once

#include <ospf/basic_definition.hpp>
#include <type_traits>
#include <format>

namespace ospf
{
    inline namespace concepts
    {
        template<typename T, typename... Args>
        struct IsSameAs;

        template<typename T, typename U>
        struct IsSameAs<T, U>
        {
            static constexpr const bool value = std::is_same_v<T, U>;
        };

        template<typename T, typename U, typename... Args>
        struct IsSameAs<T, U, Args...>
        {
            static constexpr const bool value = IsSameAs<T, U>::value || IsSameAs<T, Args...>::value;
        };

        template<typename T, typename... Args>
        static constexpr const bool is_same_as = IsSameAs<T, Args...>::value;

        template<typename T, typename... Args>
        concept SameAs = is_same_as<T, Args...>;

        template<typename T, typename U>
        concept NotSameAs = !std::is_same_v<T, U>;

        template<typename T, typename... Args>
        struct IsDecaySameAs;

        template<typename T, typename U>
        struct IsDecaySameAs<T, U>
        {
            static constexpr const bool value = std::is_same_v<std::decay_t<T>, std::decay_t<U>>;
        };

        template<typename T, typename U, typename... Args>
        struct IsDecaySameAs<T, U, Args...>
        {
            static constexpr const bool value = IsDecaySameAs<T, U>::value || IsDecaySameAs<T, Args...>::value;
        };

        template<typename T, typename... Args>
        static constexpr const bool is_decay_same_as = IsDecaySameAs<T, Args...>::value;

        template<typename T, typename... Args>
        concept DecaySameAs = is_decay_same_as<T, Args...>;

        template<typename T, typename... Args>
        concept DecayNotSameAs = !is_decay_same_as<T, Args...>;

        template<typename T, typename... Args>
        struct IsDecaySameAsOrConvertibleTo;

        template<typename T, typename U>
        struct IsDecaySameAsOrConvertibleTo<T, U>
        {
            static constexpr const bool value = std::is_same_v<std::decay_t<T>, std::decay_t<U>> || std::is_convertible_v<T, U>;
        };

        template<typename T, typename U, typename... Args>
        struct IsDecaySameAsOrConvertibleTo<T, U, Args...>
        {
            static constexpr const bool value = IsDecaySameAsOrConvertibleTo<T, U>::value || IsDecaySameAsOrConvertibleTo<T, Args...>::value;
        };

        template<typename T, typename... Args>
        static constexpr const bool is_decay_same_as_or_convertible_to = IsDecaySameAsOrConvertibleTo<T, Args...>::value;

        template<typename T, typename... Args>
        concept DecaySameAsOrConvertibleTo = is_decay_same_as_or_convertible_to<T, Args...>;

        template<typename T>
        concept NotVoidType = NotSameAs<T, void>;

        template<typename T>
        concept DecayNotVoidType = DecayNotSameAs<T, void>;

        template<typename T>
        concept CharType = SameAs<T, char, wchar, u8char, u16char, u32char>;

        template<typename T>
        concept StringType = DecaySameAs<T, std::string, std::wstring, std::u8string, std::u16string, std::u32string>;

        template<typename T>
        concept StringViewType = DecaySameAs<T, std::string_view, std::wstring_view, std::u8string_view, std::u16string_view, std::u32string_view>;

        template<typename T>
        concept StringOrViewType = StringType<T> || StringViewType<T>;

        template<StringOrViewType T>
        using CharTypeOf = typename T::value_type;
    };
};

#pragma once1

#include <ospf/string/hasher.hpp>
#include <ospf/math/symbol/expression.hpp>
#include <boost/locale.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            template<typename Self>
            class Symbol
            {
                OSPF_CRTP_IMPL;

            public:
                constexpr Symbol(std::string name)
                    : _name(std::move(name)) {}

                constexpr Symbol(std::string name, std::string display_name)
                    : _name(std::move(name)), _display_name(std::move(display_name)) {}

            public:
                constexpr Symbol(const Symbol& ano) = default;
                constexpr Symbol(Symbol&& ano) noexcept = default;
                constexpr Symbol& operator=(const Symbol& rhs) = default;
                constexpr Symbol& operator=(Symbol&& rhs) noexcept = default;
                constexpr ~Symbol(void) noexcept = default;

            public:
                inline constexpr std::string_view name(void) const noexcept
                {
                    return _name;
                }

                inline constexpr std::string_view display_name(void) const noexcept
                {
                    if (_display_name.has_value())
                    {
                        return *_display_name;
                    }
                    else
                    {
                        return _name;
                    }
                }

                inline void set_display_name(std::string new_display_name) noexcept
                {
                    _display_name = std::move(new_display_name);
                }

            public:
                inline constexpr const bool pure(void) const noexcept
                {
                    return Trait::is_pure(self());
                }

            public:
                template<typename Other>
                inline constexpr const bool operator==(const Symbol<Other>& rhs) const noexcept
                {
                    return this == &rhs || _name == rhs._name;
                }

                template<typename Other>
                inline constexpr const bool operator!=(const Symbol<Other>& rhs) const noexcept
                {
                    return this != &rhs && _name != rhs._name;
                }

            private:
                struct Trait : public Self
                {
                    inline static constexpr const bool is_pure(const Self& self) noexcept
                    {
                        static const auto impl = &Self::OSPF_CRTP_FUNCTION(is_pure);
                        return (self.*impl)();
                    }
                };

            private:
                std::string _name;
                std::optional<std::string> _display_name;
            };

            template<typename T>
            concept SymbolType = requires (const T& symbol)
            {
                { symbol.name() } -> StringViewType;
                { symbol.display_name() } -> StringViewType;
                { symbol.pure() } -> DecaySameAs<bool>;
            };

            template<typename... Ts>
            struct IsAllSymbolType;

            template<typename T>
            struct IsAllSymbolType<T>
            {
                static constexpr const bool value = SymbolType<T>;
            };

            template<typename T, typename... Ts>
            struct IsAllSymbolType<T, Ts...>
            {
                static constexpr const bool value = SymbolType<T> && IsAllSymbolType<Ts...>::value;
            };

            template<typename... Ts>
            static constexpr const bool is_all_symbol_type = IsAllSymbolType<Ts...>::value;

            template<typename... Ts>
            concept AllSymbolType = is_all_symbol_type<Ts...>;
        };
    };
};

namespace std
{
    template<typename Self, ospf::CharType CharT>
    struct formatter<ospf::Symbol<Self>, CharT>
        : public formatter<basic_string_view<CharT>, CharT>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Symbol<Self>& symbol, FormatContext& fc) const
        {
            static const auto _formatter = formatter<basic_string_view<CharT>, CharT>{};
            const auto display_name = boost::locale::conv::to_utf<CharT>(string{ symbol.display_name() }, locale{});
            return _formatter.format(display_name, fc);
        }
    };

    template<typename Self>
    struct formatter<ospf::Symbol<Self>, char>
        : public formatter<string_view, char>
    {
        template<typename FormatContext>
        inline decltype(auto) format(const ospf::Symbol<Self>& symbol, FormatContext& fc) const
        {
            static const auto _formatter = formatter<string_view, char>{};
            return _formatter.format(symbol.display_name(), fc);
        }
    };
};

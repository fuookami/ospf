#pragma once

#include <ospf/math/symbol/symbol/concepts.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace symbol
        {
            class PureSymbol
                : public Symbol<PureSymbol>
            {
                using Impl = Symbol<PureSymbol>;

            public:
                constexpr PureSymbol(std::string name)
                    : Impl(std::move(name)) {}

                constexpr PureSymbol(std::string name, std::string display_name)
                    : Impl(std::move(name), std::move(display_name)) {}

            public:
                constexpr PureSymbol(const PureSymbol& ano) = default;
                constexpr PureSymbol(PureSymbol&& ano) noexcept = default;
                constexpr PureSymbol& operator=(const PureSymbol& rhs) = default;
                constexpr PureSymbol& operator=(PureSymbol&& rhs) noexcept = default;
                constexpr ~PureSymbol(void) noexcept = default;

            OSPF_CRTP_PERMISSION:
                inline constexpr const bool OSPF_CRTP_FUNCTION(is_pure)(void) const noexcept
                {
                    return true;
                }
            };

            template<typename T>
            concept PureSymbolType = SymbolType<T> && std::is_base_of_v<PureSymbol, T>;

            template<typename... Ts>
            struct IsAllPureSymbolType;

            template<typename T>
            struct IsAllPureSymbolType<T>
            {
                static constexpr const bool value = PureSymbolType<T>;
            };

            template<typename T, typename... Ts>
            struct IsAllPureSymbolType<T, Ts...>
            {
                static constexpr const bool value = PureSymbolType<T> && IsAllPureSymbolType<Ts...>;
            };

            template<typename... Ts>
            static constexpr const bool is_all_pure_symbol_type = IsAllPureSymbolType<Ts...>::value;

            template<typename... Ts>
            concept IsAllPureSymbolType = is_all_pure_symbol_type<Ts...>;
        };
    };
};

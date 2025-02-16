#pragma once

#include <ospf/error.hpp>
#include <ospf/functional/either.hpp>

namespace ospf
{
    inline namespace functional
    {
        struct Succeed 
        {
        public:
            inline constexpr const bool operator==(const Succeed _) const noexcept
            {
                return true;
            }

            inline constexpr const bool operator!=(const Succeed _) const noexcept
            {
                return false;
            }

        public:
            inline constexpr std::strong_ordering operator<=>(const Succeed _) const noexcept
            {
                return std::strong_ordering::equal;
            }
        };
        static constexpr const auto succeed = Succeed{};

        template<typename T = Succeed, ErrorType E = OSPFError>
        class Result
        {
        public:
            using RetType = OriginType<T>;
            using ErrType = OriginType<E>;
            using EitherType = Either<RetType, ErrType>;
            using VariantType = typename EitherType::VariantType;

        public:
            inline static constexpr Result succeeded(ArgCLRefType<RetType> ret_value)
            {
                return Result{ ret_value };
            }
            
            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            inline static constexpr Result succeeded(ArgRRefType<RetType> ret_value)
            {
                return Result{ move<RetType>(ret_value) };
            }

            inline static constexpr Result failed(ArgCLRefType<ErrType> err_value)
            {
                return Result{ err_value };
            }

            template<typename = void>
                requires ReferenceFaster<ErrType> && std::movable<ErrType>
            inline static constexpr Result failed(ArgRRefType<ErrType> err_value)
            {
                return Result{ move<ErrType>(err_value) };
            }

        public:
            constexpr Result(ArgCLRefType<RetType> ret_value)
                : _either(EitherType::left(ret_value)) {}

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            Result(ArgRRefType<RetType> ret_value)
                : _either(EitherType::left(move<RetType>(ret_value))) {}

            constexpr Result(ArgCLRefType<ErrType> err_value)
                : _either(EitherType::right(err_value)) {}

            template<typename = void>
                requires ReferenceFaster<ErrType> && std::movable<ErrType>
            Result(ArgRRefType<ErrType> err_value)
                : _either(EitherType::right(move<ErrType>(err_value))) {}

            constexpr Result(const Result<T, E>& ano) = default;
            constexpr Result(Result<T, E>&& ano) noexcept = default;

        public:
            constexpr Result& operator=(const Result<T, E>& rhs) = default;
            constexpr Result& operator=(Result<T, E>&& rhs) noexcept = default;

            constexpr Result& operator=(ArgCLRefType<RetType> ret_value) noexcept
            {
                _either.assign(ret_value);
                return *this;
            }

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            Result& operator=(ArgRRefType<RetType> ret_value) noexcept
            {
                _either.assign(move<RetType>(ret_value));
                return *this;
            }

            constexpr Result& operator=(ArgCLRefType<ErrType> err_value) noexcept
            {
                _either.assign(err_value);
                return *this;
            }

            template<typename = void>
                requires ReferenceFaster<ErrType> && std::movable<ErrType>
            Result& operator=(ArgRRefType<ErrType> err_value) noexcept
            {
                _either.assign(move<ErrType>(err_value));
                return *this;
            }

        public:
            constexpr ~Result(void) noexcept = default;

        public:
            inline constexpr operator LRefType<EitherType> (void) noexcept
            {
                return _either;
            }

            inline constexpr operator CLRefType<EitherType> (void) const noexcept
            {
                return _either;
            }

            inline constexpr operator LRefType<VariantType> (void) noexcept
            {
                return static_cast<LRefType<VariantType>>(_either);
            }

            inline constexpr operator CLRefType<VariantType> (void) const noexcept
            {
                return static_cast<CLRefType<VariantType>>(_either);
            }

            inline constexpr LRefType<RetType> operator*(void) &
            {
                return _either.left();
            }

            inline constexpr CLRefType<RetType> operator*(void) const &
            {
                return _either.left();
            }

            inline ospf::RetType<RetType> operator*(void) &&
            {
                return static_cast<RRefType<EitherType>>(_either).left();
            }

        public:
            inline constexpr const bool is_succeeded(void) const noexcept
            {
                return _either.is_left();
            }

            inline constexpr const bool is_failed(void) const noexcept
            {
                return _either.is_right();
            }

        public:
            inline constexpr LRefType<RetType> unwrap(void) &
            {
                return _either.left();
            }

            inline constexpr CLRefType<RetType> unwrap(void) const &
            {
                return _either.left();
            }

            inline ospf::RetType<RetType> unwrap(void) &&
            {
                return static_cast<RRefType<EitherType>>(_either).left();
            }

            inline constexpr const std::optional<Ref<RetType>> unwrap_if_succeeded(void) & noexcept
            {
                return _either.left_if_is();
            }

            inline constexpr const std::optional<const Ref<RetType>> unwrap_if_succeeded(void) const & noexcept
            {
                return _either.left_if_is();
            }

            inline std::optional<RetType> unwrap_if_succeeded(void) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_if_is();
            }

            template<typename Func>
            inline constexpr decltype(auto) succeeded_then(Func&& func) const & noexcept
            {
                return _either.left_then(std::forward<Func>(func));
            }

            template<typename Func>
            inline decltype(auto) succeeded_then(Func&& func) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_then(std::forward<Func>(func));
            }

            inline constexpr ospf::RetType<RetType> unwrap_or(ArgCLRefType<RetType> other) const & noexcept
            {
                return _either.left_or(other);
            }

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            inline ospf::RetType<RetType> unwrap_or(ArgCLRefType<RetType> other) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_or(other);
            }

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            inline ospf::RetType<RetType> unwrap_or(ArgRRefType<RetType> other) const & noexcept
            {
                return _either.left_or(move<RetType>(other));
            }

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            inline ospf::RetType<RetType> unwrap_or(ArgRRefType<RetType> other) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_or(move<RetType>(other));
            }

            template<typename = void>
                requires WithDefault<RetType>
            inline ospf::RetType<RetType> unwrap_or_default(void) const & noexcept
            {
                return _either.left_or_default();
            }

            template<typename = void>
                requires WithDefault<RetType> && ReferenceFaster<RetType> && std::movable<RetType>
            inline ospf::RetType<RetType> unwrap_or_default(void) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_or_default();
            }

            template<typename Func>
                requires DecaySameAs<std::invoke_result_t<Func, ErrType>, RetType>
            inline ospf::RetType<RetType> unwrap_or_else(Func&& func) const & noexcept
            {
                return _either.left_or_else(std::forward<Func>(func));
            }

            template<typename Func>
                requires DecaySameAs<std::invoke_result_t<Func, ErrType>, RetType>
            inline ospf::RetType<RetType> unwrap_or_else(Func&& func) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).left_or_else(std::forward<Func>(func));
            }

        public:
            inline constexpr LRefType<ErrType> err(void) &
            {
                return _either.right();
            }

            inline constexpr CLRefType<ErrType> err(void) const &
            {
                return _either.right();
            }

            inline ospf::RetType<ErrType> err(void) &&
            {
                return static_cast<RRefType<EitherType>>(_either).right();
            }

            inline constexpr std::optional<Ref<ErrType>> err_if_is(void) & noexcept
            {
                return _either.right_if_is();
            }

            inline constexpr std::optional<const Ref<ErrType>> err_if_is(void) const & noexcept
            {
                return _either.right_if_is();
            }

            inline std::optional<ErrType> err_if_is(void) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).right_if_is();
            }

            template<typename Func>
            inline constexpr decltype(auto) err_then(Func&& func) const & noexcept
            {
                return _either.right_then(std::forward<Func>(func));
            }

            template<typename Func>
            inline decltype(auto) err_then(Func&& func) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).right_then(std::forward<Func>(func));
            }

        public:
            template<typename Func>
            inline constexpr decltype(auto) visit(Func&& func) & noexcept
            {
                return _either.visit(std::forward<Func>(func));
            }

            template<typename Func>
            inline constexpr decltype(auto) visit(Func&& func) const & noexcept
            {
                return _either.visit(std::forward<Func>(func));
            }

            template<typename Func>
            inline constexpr decltype(auto) visit(Func&& func) && noexcept
            {
                return static_cast<RRefType<EitherType>>(_either).visit(std::forward<Func>(func));
            }

        public:
            inline constexpr void reset(ArgCLRefType<RetType> ret_value) noexcept
            {
                _either.assign(ret_value);
            }

            template<typename = void>
                requires ReferenceFaster<RetType> && std::movable<RetType>
            inline void reset(ArgRRefType<RetType> ret_value) noexcept
            {
                _either.assign(move<RetType>(ret_value));
            }

            inline constexpr void reset(ArgCLRefType<ErrType> err_value) noexcept
            {
                _either.assign(err_value);
            }

            template<typename = void>
                requires ReferenceFaster<ErrType> && std::movable<ErrType>
            inline void reset(ArgRRefType<ErrType> err_value) noexcept
            {
                _either.assign(move<ErrType>(err_value));
                return *this;
            }

            template<typename... Args>
            inline constexpr decltype(auto) emplace(Args&&... args) noexcept
            {
                return _either.emplace(std::forward<Args>(args)...);
            }

            inline void swap(Result& other) noexcept
            {
                std::swap(_either, other._either);
            }

         public:
            inline constexpr const bool operator==(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() == rhs.unwrap_if_succeeded();
            }

            inline constexpr const bool operator!=(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() != rhs.unwrap_if_succeeded();
            }

        public:
            inline constexpr const bool operator<(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() < rhs.unwrap_if_succeeded();
            }

            inline constexpr const bool operator<=(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() <= rhs.unwrap_if_succeeded();
            }

            inline constexpr const bool operator>(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() > rhs.unwrap_if_succeeded();
            }

            inline constexpr const bool operator>=(const Result& rhs) const noexcept
            {
                return unwrap_if_succeeded() >= rhs.unwrap_if_succeeded();
            }

        public:
            inline constexpr decltype(auto) operator<=>(const Result& rhs) const noexcept
            {
                return this->unwrap_if_succeeded() <=> rhs.unwrap_if_succeeded();
            }

        private:
            EitherType _either;
        };

        template<ErrorType E = OSPFError>
        using Try = Result<Succeed, E>;

        extern template class Result<Succeed, OSPFError>;
    };
};

namespace std
{
    template<typename T, typename E, typename Func>
    inline constexpr decltype(auto) visit(Func&& func, ospf::Result<T, E>& result) noexcept
    {
        return result.visit(std::forward<Func>(func));
    }

    template<typename T, typename E, typename Func>
    inline constexpr decltype(auto) visit(Func&& func, const ospf::Result<T, E>& result) noexcept
    {
        return result.visit(std::forward<Func>(func));
    }

    template<typename T, typename E, typename Func>
    inline constexpr decltype(auto) visit(Func&& func, ospf::Result<T, E>&& result) noexcept
    {
        return static_cast<ospf::RRefType<ospf::Result<T, E>>>(result).visit(std::forward<Func>(func));
    }

    template<typename T, ospf::ErrorType E>
    inline void swap(ospf::Result<T, E>& lhs, ospf::Result<T, E>& rhs) noexcept
    {
        lhs.swap(rhs);
    }
};

#ifndef OSPF_TRY_AND
#define OSPF_TRY_AND(Lhs, Rhs) [&]() -> decltype(Lhs)\
{\
    auto lhs_ret = Lhs;\
    if (lhs_ret.is_failed())\
    {\
        return std::move(lhs_ret);\
    }\
    auto rhs_ret = Rhs;\
    if (rhs_ret.is_failed())\
    {\
        return std::move(rhs_ret);\
    }\
    return ospf::succeed;\
}();
#endif

#ifndef OSPF_TRY_OR
#define OSPF_TRY_OR(Lhs, Rhs) [&]() -> decltype(Lhs)\
{\
    auto lhs_ret = Lhs;\
    if (lhs_ret.is_succeeded())\
    {\
        return ospf::succeed;\
    }\
    auto rhs_ret = Rhs;\
    if (rhs_ret.is_succeeded())\
    {\
        return ospf::succeed;\
    }\
    return std::move(lhs_ret);\
}();
#endif

#ifndef OSPF_TRY_SET
#define OSPF_TRY_SET(Target, Func) auto Target##_ret = Func;\
if (Target##_ret.is_failed())\
{\
    return std::move(Target##_ret).err();\
}\
Target = std::move(Target##_ret).unwrap();
#endif

#ifndef OSPF_TRY_GET
#define OSPF_TRY_GET(Ret, Func) auto Ret##_ret = Func;\
if (Ret##_ret.is_failed())\
{\
    return std::move(Ret##_ret).err();\
}\
auto Ret = std::move(Ret##_ret).unwrap();
#endif

#ifndef OSPF_TRY_EXEC
#define OSPF_TRY_EXEC(Func) {\
    auto ret = Func;\
    if (ret.is_failed())\
    {\
        return std::move(ret).err();\
    }\
}
#endif

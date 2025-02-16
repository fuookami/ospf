#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/functional/result.hpp>
#include <ospf/functional/range_bounds.hpp>
#include <ospf/type_family.hpp>
#include <boost/iterator/transform_iterator.hpp>

namespace ospf
{
    inline namespace functional
    {
        namespace array_detail
        {
            template<
                typename T, 
                usize i, 
                usize len, 
                template<typename, usize> class C, 
                typename Func, 
                typename... Args
            >
                requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
            inline constexpr C<T, len> make_array_impl(const Func& constructor, Args&&... args) noexcept
            {
                if constexpr (i == len)
                {
                    return C<T, len>{ std::forward<Args>(args)... };
                }
                else
                {
                    return make_array_impl<T, i + 1_uz, len, C>(constructor, std::forward<Args>(args)..., constructor(i));
                }
            }

            template<
                typename T, 
                usize i, 
                usize len, 
                template<typename, usize> class C, 
                typename Func, 
                typename... Args
            >
                requires requires (const Func&& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
            inline constexpr Result<C<T, len>> try_make_array_impl(const Func& constructor, Args&&... args) noexcept
            {
                if constexpr (i == len)
                {
                    return C<T, len>{ std::forward<Args>(args)... };
                }
                else
                {
                    OSPF_TRY_GET(this_arg, constructor(i));
                    return try_make_array_impl<T, i + 1_uz, len, C>(constructor, std::forward<Args>(args)..., std::move(this_arg));
                }
            }

            template<
                typename T, 
                template<typename> class C, 
                typename Func
            >
                requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
            inline constexpr C<T> make_array_impl(const usize len, const Func& constructor) noexcept
            {
                const auto range = 0_uz RTo len;
                return C<T>
                {
                    boost::make_transform_iterator(range.begin(), constructor),
                    boost::make_transform_iterator(range.end(), constructor)
                };
            }

            template<
                typename T, 
                template<typename> class C, 
                typename Func
            >
                requires requires (const Func&& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
            inline constexpr C<T> try_make_array_impl(const usize len, const Func& constructor) noexcept
            {
                const auto trans_func = [&constructor](const usize i) -> RetType<T>
                {
                    const auto this_arg = constructor(i);
                    if (this_arg.is_failed())
                    {
                        throw OSPFException{ std::move(this_arg).err() };
                    }
                    return std::move(this_arg).unwrap();
                };
                const auto range = 0_uz RTo len;
                try
                {
                    return C<T>
                    {
                        boost::make_transform_iterator(range.begin(), trans_func),
                        boost::make_transform_iterator(range.end(), trans_func)
                    };
                }
                catch (OSPFException& e)
                {
                    return std::move(e).error();
                }
            }
        };

        template<
            typename T, 
            usize len, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<T>; }
        inline constexpr std::array<T, len> make_array(It&& it) noexcept
        {
            return array_detail::make_array_impl<T, 0_uz, len, std::array>([&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            usize len, 
            template<typename, usize> class C, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<T>; }
        inline constexpr C<T, len> make_array(It&& it) noexcept
        {
            return array_detail::make_array_impl<T, 0_uz, len, C>([&it](const usize _)
                {
                    return *it;
                });
        }
        
        template<
            typename T, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<T>; }
        inline constexpr std::vector<T> make_array(const usize len, It&& it) noexcept
        {
            return array_detail::make_array_impl<T, std::vector>(len, [&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            template<typename> class C, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<T>; }
        inline constexpr C<T> make_array(const usize len, It&& it) noexcept
        {
            return array_detail::make_array_impl<T, C>(len, [&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            usize len, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<std::array<T, len>> make_array(It&& it) noexcept
        {
            return array_detail::try_make_array_impl<T, 0_uz, len, std::array>([&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            usize len, 
            template<typename, usize> class C, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<C<T, len>> make_array(It&& it) noexcept
        {
            return array_detail::try_make_array_impl<T, 0_uz, len, C>([&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<std::vector<T>> make_array(const usize len, It&& it) noexcept
        {
            return array_detail::try_make_array_impl<T, std::vector>(len, [&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            template<typename> class C, 
            std::input_iterator It
        >
            requires requires (const It& it) { { *it } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<C<T>> make_array(const usize len, It&& it) noexcept
        {
            return array_detail::try_make_array_impl<T, C>(len, [&it](const usize _)
                {
                    return *it;
                });
        }

        template<
            typename T, 
            usize len, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
        inline constexpr std::array<T, len> make_array(const Func& constructor) noexcept
        {
            return array_detail::make_array_impl<T, 0_uz, len, std::array>(constructor);
        }

        template<
            typename T, 
            usize len, 
            template<typename, usize> class C, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
        inline constexpr C<T, len> make_array(const Func& constructor) noexcept
        {
            return array_detail::make_array_impl<T, 0_uz, len, C>(constructor);
        }

        template<
            typename T, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
        inline constexpr std::vector<T> make_array(const usize len, const Func& constructor) noexcept
        {
            return array_detail::make_array_impl<T, std::vector>(len, constructor);
        }

        template<
            typename T, 
            template<typename> class C, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<T>; }
        inline constexpr C<T> make_array(const usize len, const Func& constructor) noexcept
        {
            return array_detail::make_array_impl<T, C>(len, constructor);
        }

        template<
            typename T, 
            usize len, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<std::array<T, len>> make_array(const Func& constructor) noexcept
        {
            return array_detail::try_make_array_impl<T, 0_uz, len, std::array>(constructor);
        }

        template<
            typename T, 
            usize len, 
            template<typename, usize> class C, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<C<T, len>> make_array(const Func& constructor) noexcept
        {
            return array_detail::try_make_array_impl<T, 0_uz, len, C>(constructor);
        }

        template<
            typename T, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<std::vector<T>> make_array(const usize len, const Func& constructor) noexcept
        {
            return array_detail::try_make_array_impl<T, std::vector>(len, constructor);
        }

        template<
            typename T, 
            template<typename> class C, 
            typename Func
        >
            requires requires (const Func& func) { { func(std::declval<usize>()) } -> DecaySameAs<Result<T>>; }
        inline constexpr Result<C<T>> make_array(const usize len, const Func& constructor) noexcept
        {
            return array_detail::try_make_array_impl<T, C>(len, constructor);
        }

        template<
            typename T, 
            usize len, 
            template<typename, usize> class C = std::array
        >
            requires std::copyable<T>
        inline constexpr C<T, len> make_array(ArgCLRefType<T> value) noexcept
        {
            if constexpr (CopyFaster<T>)
            {
                return array_detail::make_array_impl<T, 0_uz, len, C>([value](const usize _)
                    {
                        return value;
                    });
            }
            else
            {
                return array_detail::make_array_impl<T, 0_uz, len, C>([&value](const usize _)
                    {
                        return value;
                    });
            }
        }

        template<
            typename T, 
            template<typename> class C = std::vector
        >
            requires std::copyable<T>
        inline constexpr C<T> make_array(const usize len, ArgCLRefType<T> value) noexcept
        {
            if constexpr (CopyFaster<T>)
            {
                return array_detail::make_array_impl<T, C>(len, [value](const usize _)
                    {
                        return value;
                    });
            }
            else
            {
                return array_detail::make_array_impl<T, C>(len, [&value](const usize _)
                    {
                        return value;
                    });
            }
        }
    };
};

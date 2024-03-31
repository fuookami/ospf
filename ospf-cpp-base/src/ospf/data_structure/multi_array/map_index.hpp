#pragma once

#include <ospf/data_structure/multi_array/dummy_index.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace multi_array
        {
            namespace map_index
            {
                using RangeFull = range_bounds::RangeFull;
                using DummyIndex = dummy_index::DummyIndex;

                struct MapPlaceHolder
                {
                    usize to_dimension;

                    inline constexpr operator const usize(void) const noexcept
                    {
                        return to_dimension;
                    }
                };

                static constexpr const auto _0 = MapPlaceHolder{ 0_uz };
                static constexpr const auto _1 = MapPlaceHolder{ 1_uz };
                static constexpr const auto _2 = MapPlaceHolder{ 2_uz };
                static constexpr const auto _3 = MapPlaceHolder{ 3_uz };
                static constexpr const auto _4 = MapPlaceHolder{ 4_uz };
                static constexpr const auto _5 = MapPlaceHolder{ 5_uz };
                static constexpr const auto _6 = MapPlaceHolder{ 6_uz };
                static constexpr const auto _7 = MapPlaceHolder{ 7_uz };
                static constexpr const auto _8 = MapPlaceHolder{ 8_uz };
                static constexpr const auto _9 = MapPlaceHolder{ 9_uz };
                static constexpr const auto _10 = MapPlaceHolder{ 10_uz };
                static constexpr const auto _11 = MapPlaceHolder{ 11_uz };
                static constexpr const auto _12 = MapPlaceHolder{ 12_uz };
                static constexpr const auto _13 = MapPlaceHolder{ 13_uz };
                static constexpr const auto _14 = MapPlaceHolder{ 14_uz };
                static constexpr const auto _15 = MapPlaceHolder{ 15_uz };
                static constexpr const auto _16 = MapPlaceHolder{ 16_uz };
                static constexpr const auto _17 = MapPlaceHolder{ 17_uz };
                static constexpr const auto _18 = MapPlaceHolder{ 18_uz };
                static constexpr const auto _19 = MapPlaceHolder{ 19_uz };
                static constexpr const auto _20 = MapPlaceHolder{ 20_uz };

                class MapIndex
                {
                    using Either = functional::Either<DummyIndex, MapPlaceHolder>;

                public:
                    constexpr MapIndex(const usize index)
                        : _either(DummyIndex{ index }) {}

                    constexpr MapIndex(const isize index)
                        : _either(DummyIndex{ index }) {}

                    constexpr MapIndex(const RangeBounds<usize> range)
                        : _either(DummyIndex{ range }) {}

                    constexpr MapIndex(const RangeBounds<isize> range)
                        : _either(DummyIndex{ range }) {}

                    constexpr MapIndex(const std::vector<usize>& array)
                        : _either(DummyIndex{ array }) {}

                    constexpr MapIndex(std::vector<usize>&& array)
                        : _either(DummyIndex{ std::move(array) }) {}

                    constexpr MapIndex(const std::vector<isize>& array)
                        : _either(DummyIndex{ array }) {}

                    constexpr MapIndex(std::vector<isize>&& array)
                        : _either(DummyIndex{ std::move(array) }) {}

                    constexpr MapIndex(const RangeFull _)
                        : _either(DummyIndex{ _ }) {}

                    template<typename T>
                        requires std::constructible_from<DummyIndex, T>
                    constexpr MapIndex(T&& index)
                        : _either(DummyIndex{ std::forward<T>(index) }) {}

                    constexpr MapIndex(const MapPlaceHolder holder)
                        : _either(holder) {}

                public:
                    constexpr MapIndex(const MapIndex& ano) = default;
                    constexpr MapIndex(MapIndex&& ano) noexcept = default;
                    constexpr MapIndex& operator=(const MapIndex& rhs) = default;
                    constexpr MapIndex& operator=(MapIndex&& rhs) noexcept = default;
                    constexpr ~MapIndex(void) noexcept = default;

                public:
                    inline constexpr const bool is_index(void) const noexcept
                    {
                        return _either.is_left();
                    }

                    inline constexpr const bool is_holder(void) const noexcept
                    {
                        return _either.is_right();
                    }

                public:
                    inline constexpr const DummyIndex& index(void) const &
                    {
                        return _either.left();
                    }

                    inline RetType<DummyIndex> index(void) &&
                    {
                        return static_cast<Either&&>(_either).left();
                    }

                    inline constexpr const MapPlaceHolder holder(void) const
                    {
                        return _either.right();
                    }

                private:
                    Either _either;
                };

                template<usize dim, usize to_dim>
                    requires (dim > to_dim) && (to_dim != 0_uz)
                class MapVector
                {
                    using Index = map_index::MapIndex;

                public:
                    constexpr MapVector(std::initializer_list<Index> vector)
                        : _vector(std::move(vector)) {}
                    constexpr MapVector(const MapVector& ano) = default;
                    constexpr MapVector(MapVector&& ano) noexcept = default;
                    constexpr MapVector& operator=(const MapVector& rhs) = default;
                    constexpr MapVector& operator=(MapVector&& rhs) noexcept = default;
                    constexpr ~MapVector(void) noexcept = default;

                public:
                    inline constexpr ArgCLRefType<Index> operator[](const usize i) const
                    {
                        return _vector[i];
                    }

                public:
                    inline constexpr const usize size(void) const noexcept
                    {
                        return dim;
                    }

                private:
                    std::array<Index, dim> _vector;
                };
            };

            template<typename... Args>
            struct IsMapVector
            {
                static constexpr const bool value = false;
            };

            template<typename T>
            struct IsMapVector<T>
            {
                static constexpr const bool value = dummy_index::DummyIndex::Types::template contains<T> || DecaySameAs<T, map_index::MapPlaceHolder>;
            };

            template<typename T, typename... Args>
            struct IsMapVector<T, Args...>
            {
                static constexpr const bool value = IsMapVector<T>::value && IsMapVector<Args...>::value;
            };

            template<typename... Args>
            static constexpr const bool is_map_vector = IsMapVector<Args...>::value;

            template<typename... Args>
                requires is_map_vector<Args...>
            inline constexpr decltype(auto) map_vector(Args&&... args) noexcept
            {
                static constexpr const auto dim = VariableTypeList<Args...>::length;
                static constexpr const auto to_dim = VariableTypeList<Args...>::template count<map_index::MapPlaceHolder>;
                static_assert(to_dim != 0_uz, "there should be at least 1 place holders in map vector!");
                return map_index::MapVector<dim, to_dim>{ std::forward<map_index::MapIndex>(args)... };
            }

            template<usize vec_dim, usize to_dim>
            inline constexpr std::array<usize, to_dim> map_to_dimension(const map_index::MapVector<vec_dim, to_dim>& vector)
            {
                std::array<std::pair<usize, usize>, to_dim> map{ { 0_uz, 0_uz } };
                for (usize i{ 0_uz }, j{ 0_uz }; i != vec_dim; ++i)
                {
                    if (vector[i].is_holder())
                    {
                        map[j] = std::make_pair(i, vector[i].holder().to_dimension);
                        ++j;
                    }
                }
                std::sort(map.begin(), map.end(),
                    [](const std::pair<usize, usize> lhs, const std::pair<usize, usize> rhs)
                    {
                        return lhs.second < rhs.second;
                    });
                for (usize i{ 0_uz }, j{ to_dim - 1_uz }; i != j; ++i)
                {
                    if (map[i].second == map[i + 1_uz].second)
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("same mapping to dimension between dimension {} and {}", map[i].first, map[i + 1_uz].first) };
                    }
                }
                std::array<usize, to_dim> ret{ 0_uz };
                for (usize i{ 0_uz }; i != to_dim; ++i)
                {
                    ret[i] = map[i].first;
                }
                return ret;
            }

            template<ShapeType S, usize to_dim>
            inline constexpr decltype(auto) map_to_shape(const S& shape, const std::array<usize, to_dim>& map_dimension)
            {
                using ToShapeType = Shape<to_dim>;
                using ToVectorType = typename ToShapeType::VectorType;
                ToVectorType ret{ 0_uz };
                for (usize i{ 0_uz }; i != to_dim; ++i)
                {
                    ret[i] = shape.size_of_dimension(map_dimension[i]);
                }
                return ToShapeType{ move<ToVectorType>(ret) };
            }

            template<ShapeType S, usize vec_dim, usize to_dim>
            inline static constexpr decltype(auto) base_map_to_vector(const S& shape, const map_index::MapVector<vec_dim, to_dim>& vector)
            {
                using DummyVectorType = typename S::DummyVectorType;

                if constexpr (S::dim == dynamic_dimension)
                {
                    DummyVectorType ret{ shape.dimension(), dummy_index::DummyIndex{} };
                    for (usize i{ 0_uz }; i != shape.dimension(); ++i)
                    {
                        if (vector[i].is_index())
                        {
                            ret[i] = vector[i].index();
                        }
                    }
                    return ret;
                }
                else
                {
                    DummyVectorType ret{ dummy_index::DummyIndex{} };
                    for (usize i{ 0_uz }; i != shape.dimension(); ++i)
                    {
                        if (vector[i].is_index())
                        {
                            ret[i] = vector[i].index();
                        }
                    }
                    return ret;
                }
            }

            template<ShapeType S, usize vec_dim, usize to_dim>
            inline static constexpr decltype(auto) base_map_to_vector(const S& shape, map_index::MapVector<vec_dim, to_dim>&& vector)
            {
                using DummyVectorType = typename S::DummyVectorType;

                if constexpr (S::dim == dynamic_dimension)
                {
                    DummyVectorType ret{ shape.dimension(), dummy_index::DummyIndex{} };
                    for (usize i{ 0_uz }; i != shape.dimension(); ++i)
                    {
                        if (vector[i].is_index())
                        {
                            ret[i] = std::move(vector[i]).index();
                        }
                    }
                    return ret;
                }
                else
                {
                    DummyVectorType ret{ dummy_index::DummyIndex{} };
                    for (usize i{ 0_uz }; i != shape.dimension(); ++i)
                    {
                        if (vector[i].is_index())
                        {
                            ret[i] = std::move(vector[i]).index();
                        }
                    }
                    return ret;
                }
            }
        };
    };
};

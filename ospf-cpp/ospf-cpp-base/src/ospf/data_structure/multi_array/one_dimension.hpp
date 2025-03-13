#pragma once

#include <ospf/data_structure/multi_array/concepts.hpp>
#include <ospf/data_structure/multi_array/impl.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace multi_array
        {
            template<typename T>
                requires NotSameAs<T, void>
            class MultiArray<T, 1_uz>
                : public MultiArrayImpl<OriginType<T>, std::vector<OriginType<T>>, Shape1, MultiArray<T, 1_uz>>
            {
                using Impl = MultiArrayImpl<OriginType<T>, std::vector<OriginType<T>>, Shape1, MultiArray<T, 1_uz>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::ContainerType;
                using typename Impl::ShapeType;
                using typename Impl::VectorType;
                using typename Impl::VectorViewType;
                using typename Impl::DummyVectorType;
                using typename Impl::DummyVectorViewType;

            public:
                template<typename = void>
                    requires WithDefault<ValueType>
                constexpr MultiArray(ArgRRefType<Shape1> shape)
                    : _shape(move<Shape1>(shape))
                {
                    _container.assign(_shape.size(), DefaultValue<ValueType>::value());
                }

                constexpr MultiArray(ArgRRefType<Shape1> shape, ArgCLRefType<ValueType> value)
                    : _shape(move<Shape1>(shape))
                {
                    _container.assign(_shape.size(), value);
                }

                template<typename F>
                    requires requires (const F& fun, const usize i) { { fun(i) } -> DecaySameAs<ValueType>; }
                constexpr MultiArray(ArgRRefType<Shape1> shape, const F& constructor)
                    : _shape(move<Shape1>(shape))
                {
                    _container.reserve(_shape.size());
                    for (usize i{ 0_uz }; i != _shape.size(); ++i)
                    {
                        _container.push_back(constructor(i));
                    }
                }

                template<typename F>
                    requires requires (const F& fun, const VectorType& vec) { { fun(vec) } -> DecaySameAs<ValueType>; }
                constexpr MultiArray(ArgRRefType<Shape1> shape, const F& constructor)
                    : _shape(move<Shape1>(shape))
                {
                    _container.reserve(_shape.size());
                    auto vector = _shape.zero();
                    do
                    {
                        _container.push_back(constructor(vector));
                    } while (_shape.next_vector(vector));
                }

            public:
                constexpr MultiArray(const MultiArray& ano) = default;
                constexpr MultiArray(MultiArray&& ano) noexcept = default;
                constexpr MultiArray& operator=(const MultiArray& rhs) = default;
                constexpr MultiArray& operator=(MultiArray&& rhs) noexcept = default;
                constexpr ~MultiArray(void) noexcept = default;

            public:
                using Impl::operator[];

                inline constexpr LRefType<ValueType> operator[](const usize i) noexcept
                {
                    assert(i == 0_uz);
                    return this->get(i);
                }

                inline constexpr CLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    assert(i == 0_uz);
                    return this->get(i);
                }

            public:
                // todo: reshape or resize

            OSPF_CRTP_PERMISSION:
                inline constexpr CLRefType<ShapeType> OSPF_CRTP_FUNCTION(get_shape)(void) const noexcept
                {
                    return _shape;
                }

                inline constexpr LRefType<ContainerType> OSPF_CRTP_FUNCTION(get_container)(void) noexcept
                {
                    return _container;
                }

                inline constexpr CLRefType<ContainerType> OSPF_CRTP_FUNCTION(get_const_container)(void) const noexcept
                {
                    return _container;
                }

                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get_value)(LRefType<ContainerType>& array, const usize i) noexcept
                {
                    return array[i];
                }

                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get_const_value)(CLRefType<ContainerType>& array, const usize i) noexcept
                {
                    return array[i];
                }

            private:
                Shape1 _shape;
                std::vector<T> _container;
            };

            template<typename T>
                requires NotSameAs<T, void>
            class MultiArray<T, 0_uz>
                : public MultiArrayImpl<T, std::array<T, 1_uz>, Shape<0_uz>, MultiArray<T, 0_uz>>
            {
                using Impl = MultiArrayImpl<T, std::array<T, 1_uz>, Shape<0_uz>, MultiArray<T, 0_uz>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::ContainerType;
                using typename Impl::ShapeType;
                using typename Impl::VectorType;
                using typename Impl::VectorViewType;
                using typename Impl::DummyVectorType;
                using typename Impl::DummyVectorViewType;

            public:
                template<typename = void>
                    requires WithDefault<ValueType>
                constexpr MultiArray(void)
                    : _container(DefaultValue<ValueType>::value()) {}

                constexpr MultiArray(ArgRRefType<ValueType> value)
                    : _container(move<ValueType>(value)) {}

            public:
                constexpr MultiArray(const MultiArray& ano) = default;
                constexpr MultiArray(MultiArray&& ano) noexcept = default;
                constexpr MultiArray& operator=(const MultiArray& rhs) = default;
                constexpr MultiArray& operator=(MultiArray&& rhs) noexcept = default;
                constexpr ~MultiArray(void) noexcept = default;

            public:
                using Impl::operator[];

                inline constexpr LRefType<ValueType> operator[](const usize i) noexcept
                {
                    assert(i == 0_uz);
                    return this->get(i);
                }

                inline constexpr CLRefType<ValueType> operator[](const usize i) const noexcept
                {
                    assert(i == 0_uz);
                    return this->get(i);
                }

            OSPF_CRTP_PERMISSION:
                inline constexpr CLRefType<ShapeType> OSPF_CRTP_FUNCTION(get_shape)(void) const noexcept
                {
                    return _shape;
                }

                inline constexpr LRefType<ContainerType> OSPF_CRTP_FUNCTION(get_container)(void) noexcept
                {
                    return _container;
                }

                inline constexpr CLRefType<ContainerType> OSPF_CRTP_FUNCTION(get_const_container)(void) const noexcept
                {
                    return _container;
                }

                inline static constexpr LRefType<ValueType> OSPF_CRTP_FUNCTION(get_value)(LRefType<ContainerType>& array, const usize i) noexcept
                {
                    return array[i];
                }

                inline static constexpr CLRefType<ValueType> OSPF_CRTP_FUNCTION(get_const_value)(CLRefType<ContainerType>& array, const usize i) noexcept
                {
                    return array[i];
                }

            private:
                Shape<0_uz> _shape;
                std::array<T, 1_uz> _container;
            };
        };
    };
};

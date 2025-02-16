#pragma once

#include <ospf/data_structure/multi_array/concepts.hpp>
#include <ospf/data_structure/multi_array/impl.hpp>
#include <ospf/data_structure/multi_array/map_view.hpp>
#include <ospf/data_structure/multi_array/view.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace multi_array
        {
            template<
                typename T,
                usize dim
            >
                requires NotSameAs<T, void>
            class MultiArray
                : public MultiArrayImpl<OriginType<T>, std::vector<OriginType<T>>, Shape<dim>, MultiArray<T, dim>>
            {
                using Impl = MultiArrayImpl<OriginType<T>, std::vector<OriginType<T>>, Shape<dim>, MultiArray<T, dim>>;

            public:
                using typename Impl::ValueType;
                using typename Impl::ContainerType;
                using typename Impl::ShapeType;
                using typename Impl::VectorType;
                using typename Impl::VectorViewType;
                using typename Impl::DummyVectorType;
                using typename Impl::DummyVectorViewType;

                template<usize to_dim>
                using MapVectorType = map_index::MapVector<dim, to_dim>;

            public:
                template<typename = void>
                    requires WithDefault<ValueType>
                constexpr MultiArray(ArgRRefType<ShapeType> shape)
                    : _shape(move<ShapeType>(shape))
                {
                    _container.assign(_shape.size(), DefaultValue<ValueType>::value());
                }

                constexpr MultiArray(ArgRRefType<ShapeType> shape, ArgCLRefType<ValueType> value)
                    : _shape(move<ShapeType>(shape))
                {
                    _container.assign(_shape.size(), value);
                }

                template<typename F>
                    requires requires (const F& fun, const usize i) { { fun(i) } -> DecaySameAs<ValueType>; }
                constexpr MultiArray(ArgRRefType<ShapeType> shape, const F& constructor)
                    : _shape(move<ShapeType>(shape))
                {
                    _container.reserve(_shape.size());
                    for (usize i{ 0_uz }; i != _shape.size(); ++i)
                    {
                        _container.push_back(constructor(i));
                    }
                }

                template<typename F>
                    requires requires (const F& fun, const VectorType& vec) { { fun(vec) } -> DecaySameAs<ValueType>; }
                constexpr MultiArray(ArgRRefType<ShapeType> shape, const F& constructor)
                    : _shape(move<ShapeType>(shape))
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
                using Impl::get;

                template<usize to_dim>
                    requires (to_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) get(const MapVectorType<to_dim>& vector) const
                {
                    auto [shape, container] = this->template map<DynRefArray<DynRefArray<ValueType>>>(this->shape(), vector);
                    return MultiArrayMapView<ValueType, to_dim>{ std::move(shape), std::move(container) };
                }

                template<usize to_dim>
                    requires (to_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) get(MapVectorType<to_dim>&& vector) const
                {
                    auto [shape, container] = this->template map<DynRefArray<DynRefArray<ValueType>>>(this->shape(), std::move(vector));
                    return MultiArrayMapView<ValueType, to_dim>{ std::move(shape), std::move(container) };
                }

            public:
                using Impl::operator();

                // todo: use operator[...] to replace operator(...) in C++23

                template<typename... Args>
                    requires is_map_vector<Args...> && !is_dummy_vector<Args...> && !is_normal_vector<Args...>
                inline constexpr decltype(auto) operator()(Args&&... args) const
                {
                    return get(map_vector(std::forward<Args>(args)...));
                }

            public:
                using Impl::operator[];

                template<usize to_dim>
                    requires (to_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) operator[](const MapVectorType<to_dim>& vector) const
                {
                    return get(vector);
                }

                template<usize to_dim>
                    requires (to_dim < dim) && (to_dim != 0_uz)
                inline constexpr decltype(auto) operator[](MapVectorType<to_dim>&& vector) const
                {
                    return get(std::move(vector));
                }

            public:
                inline constexpr MultiArrayView<MultiArray, DynShape> view(void) const
                {
                    return MultiArrayView<MultiArray, DynShape>{ *this, make_array<DummyIndex, dim>(DummyIndex{}) };
                }

                template<typename... Args>
                    requires (VariableTypeList<Args...>::length != 0_uz) && (VariableTypeList<Args...>::length <= dim) && is_view_vector<Args...>
                inline constexpr MultiArrayView<MultiArray, DynShape> view(Args&&... args) const
                {
                    static constexpr const usize to_dim = VariableTypeList<Args...>::length;
                    auto vector = make_array<DummyIndex, dim>(DummyIndex{});
                    MultiArrayView<MultiArray, DynShape>::template view_vector<0_uz, to_dim>(vector, std::forward<Args>(args)...);
                    return MultiArrayView<MultiArray, DynShape>{ *this, std::move(vector) };
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
                ShapeType _shape;
                std::vector<ValueType> _container;
            };
        };
    };
};

#pragma once

#include <ospf/data_structure/multi_array/concepts.hpp>
#include <ospf/data_structure/multi_array/impl.hpp>
#include <ospf/data_structure/multi_array/map_view.hpp>

namespace ospf
{
    inline namespace data_structure
    {
        namespace multi_array
        {
            template<typename... Args>
            struct IsViewVector
            {
                static constexpr const bool value = false;
            };

            template<typename T>
            struct IsViewVector<T>
            {
                static constexpr const bool value = DecaySameAs<T, usize, isize, RangeFull> || std::integral<T>;
            };

            template<typename T, typename... Args>
            struct IsViewVector<T, Args...>
            {
                static constexpr const bool value = IsViewVector<T>::value && IsViewVector<Args...>::value;
            };

            template<typename... Args>
            static constexpr const bool is_view_vector = IsViewVector<Args...>::value;

            template<typename V>
            class MultiArrayViewConstIterator
            {
            public:
                using ViewType = OriginType<V>;
                using ValueType = typename ViewType::ValueType;
                using ShapeType = typename ViewType::ShapeType;
                using VectorType = typename ViewType::VectorType;

            public:
                constexpr MultiArrayViewConstIterator(const ViewType& view)
                    : _has_next(false), _view(view) {}

                constexpr MultiArrayViewConstIterator(ArgRRefType<VectorType> vector, const ViewType& view)
                    : _has_next(true), _vector(move<ValueType>(vector)), _view(view) {}

            public:
                constexpr MultiArrayViewConstIterator(const MultiArrayViewConstIterator& ano) = default;
                constexpr MultiArrayViewConstIterator(MultiArrayViewConstIterator&& rhs) noexcept = default;
                constexpr MultiArrayViewConstIterator& operator=(const MultiArrayViewConstIterator& rhs) = default;
                constexpr MultiArrayViewConstIterator& operator=(MultiArrayViewConstIterator&& rhs) noexcept = default;
                constexpr ~MultiArrayViewConstIterator(void) noexcept = default;

            public:
                inline constexpr CLRefType<ValueType> operator*(void) const noexcept
                {
                    return _view->get(_vector);
                }

                inline constexpr CPtrType<ValueType> operator->(void) const noexcept
                {
                    return &_view->get(_vector);
                }

            public:
                inline constexpr MultiArrayViewConstIterator& operator++(void) noexcept
                {
                    next();
                    return *this;
                }

                inline constexpr MultiArrayViewConstIterator operator++(int) noexcept
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

            public:
                inline constexpr const bool operator==(const MultiArrayViewConstIterator& rhs) const noexcept
                {
                    if (_view != rhs._view)
                    {
                        return false;
                    }
                    if (!_has_next && !rhs._has_next)
                    {
                        return true;
                    }
                    else if (_has_next && rhs._has_next)
                    {
                        return _vector == rhs._vector;
                    }
                    else
                    {
                        return false;
                    }
                }

                inline constexpr const bool operator!=(const MultiArrayViewConstIterator& rhs) const noexcept
                {
                    if (_view == rhs._view)
                    {
                        return false;
                    }
                    if (!_has_next && !rhs._has_next)
                    {
                        return false;
                    }
                    else if (_has_next && rhs._has_next)
                    {
                        return _vector =! rhs._vector;
                    }
                    else
                    {
                        return true;
                    }
                }

            public:
                inline void swap(MultiArrayViewConstIterator& rhs) noexcept
                {
                    std::swap(_has_next, rhs._has_next);
                    std::swap(_vector, rhs._vector);
                    std::swap(_view, rhs._view);
                }

            protected:
                inline constexpr void next(void) noexcept
                {
                    if (!_view->shape().next_vector(_vector))
                    {
                        _has_next = false;
                    }
                }

            private:
                bool _has_next;
                VectorType _vector;
                Ref<ViewType> _view;
            };

            template<typename V>
            class MultiArrayViewIterator
                : public MultiArrayViewConstIterator<OriginType<V>>
            {
                using Base = MultiArrayViewConstIterator<OriginType<V>>;

            public:
                using ViewType = typename Base::ViewType;
                using ValueType = typename Base::ValueType;
                using ShapeType = typename Base::ShapeType;
                using VectorType = typename Base::VectorType;

            public:
                constexpr MultiArrayViewIterator(const ViewType& view)
                    : Base(view) {}

                constexpr MultiArrayViewIterator(ArgRRefType<VectorType> vector, const ViewType& view)
                    : Base(move<VectorType>(vector), view) {}

            public:
                constexpr MultiArrayViewIterator(const MultiArrayViewIterator& ano) = default;
                constexpr MultiArrayViewIterator(MultiArrayViewIterator&& rhs) noexcept = default;
                constexpr MultiArrayViewIterator& operator=(const MultiArrayViewIterator& rhs) = default;
                constexpr MultiArrayViewIterator& operator=(MultiArrayViewIterator&& rhs) noexcept = default;
                constexpr ~MultiArrayViewIterator(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> operator*(void) const noexcept
                {
                    return const_cast<LRefType<ValueType>>(Base::operator*());
                }

                inline constexpr PtrType<ValueType> operator->(void) const noexcept
                {
                    return const_cast<PtrType<ValueType>>(Base::operator->());
                }

            public:
                inline constexpr MultiArrayViewIterator& operator++(void) noexcept
                {
                    Base::next();
                    return *this;
                }

                inline constexpr const MultiArrayViewIterator operator++(int) noexcept
                {
                    auto ret = *this;
                    Base::next();
                    return ret;
                }
            };

            template<typename V>
            class MultiArrayViewConstReverseIterator
            {
            public:
                using ViewType = OriginType<V>;
                using ValueType = typename ViewType::ValueType;
                using ShapeType = typename ViewType::ShapeType;
                using VectorType = typename ViewType::VectorType;

            public:
                constexpr MultiArrayViewConstReverseIterator(const ViewType& view)
                    : _has_next(false), _view(view) {}

                constexpr MultiArrayViewConstReverseIterator(ArgRRefType<VectorType> vector, const ViewType& view)
                    : _has_next(true), _vector(move<ValueType>(vector)), _view(view) {}

            public:
                constexpr MultiArrayViewConstReverseIterator(const MultiArrayViewConstReverseIterator& ano) = default;
                constexpr MultiArrayViewConstReverseIterator(MultiArrayViewConstReverseIterator&& rhs) noexcept = default;
                constexpr MultiArrayViewConstReverseIterator& operator=(const MultiArrayViewConstReverseIterator& rhs) = default;
                constexpr MultiArrayViewConstReverseIterator& operator=(MultiArrayViewConstReverseIterator&& rhs) noexcept = default;
                constexpr ~MultiArrayViewConstReverseIterator(void) noexcept = default;

            public:
                inline constexpr CLRefType<ValueType> operator*(void) const noexcept
                {
                    return _view->get(_vector);
                }

                inline constexpr CPtrType<ValueType> operator->(void) const noexcept
                {
                    return &_view->get(_vector);
                }

            public:
                inline constexpr MultiArrayViewConstReverseIterator& operator++(void) noexcept
                {
                    next();
                    return *this;
                }

                inline constexpr const MultiArrayViewConstReverseIterator operator++(int) noexcept
                {
                    auto ret = *this;
                    next();
                    return ret;
                }

            public:
                inline constexpr const bool operator==(const MultiArrayViewConstReverseIterator& rhs) const noexcept
                {
                    if (_view != rhs._view)
                    {
                        return false;
                    }
                    if (!_has_next && !rhs._has_next)
                    {
                        return true;
                    }
                    else if (_has_next && rhs._has_next)
                    {
                        return _vector == rhs._vector;
                    }
                    else
                    {
                        return false;
                    }
                }

                inline constexpr const bool operator!=(const MultiArrayViewConstReverseIterator& rhs) const noexcept
                {
                    if (_view == rhs._view)
                    {
                        return false;
                    }
                    if (!_has_next && !rhs._has_next)
                    {
                        return false;
                    }
                    else if (_has_next && rhs._has_next)
                    {
                        return _vector = !rhs._vector;
                    }
                    else
                    {
                        return true;
                    }
                }

            public:
                inline void swap(MultiArrayViewConstReverseIterator& rhs) noexcept
                {
                    std::swap(_has_next, rhs._has_next);
                    std::swap(_vector, rhs._vector);
                    std::swap(_view, rhs._view);
                }

            protected:
                inline constexpr void next(void) noexcept
                {
                    if (!_view->shape().last_vector(_vector))
                    {
                        _has_next = false;
                    }
                }

            private:
                bool _has_next; 
                VectorType _vector;
                Ref<ViewType> _view;
            };

            template<typename V>
            class MultiArrayViewReverseIterator
                : public MultiArrayViewConstReverseIterator<OriginType<V>>
            {
                using Base = MultiArrayViewConstReverseIterator<OriginType<V>>;

            public:
                using ViewType = typename Base::ViewType;
                using ValueType = typename Base::ValueType;
                using ShapeType = typename Base::ShapeType;
                using VectorType = typename Base::VectorType;

            public:
                constexpr MultiArrayViewReverseIterator(const ViewType& view)
                    : Base(view) {}

                constexpr MultiArrayViewReverseIterator(ArgRRefType<VectorType> vector, const ViewType& view)
                    : Base(move<VectorType>(vector), view) {}

            public:
                constexpr MultiArrayViewReverseIterator(const MultiArrayViewReverseIterator& ano) = default;
                constexpr MultiArrayViewReverseIterator(MultiArrayViewReverseIterator&& rhs) noexcept = default;
                constexpr MultiArrayViewReverseIterator& operator=(const MultiArrayViewReverseIterator& rhs) = default;
                constexpr MultiArrayViewReverseIterator& operator=(MultiArrayViewReverseIterator&& rhs) noexcept = default;
                constexpr ~MultiArrayViewReverseIterator(void) noexcept = default;
            };

            template<
                typename A,
                typename S
            >
                requires NotSameAs<typename A::ValueType, void>
            class MultiArrayView
            {
                template<
                    typename T,
                    usize dim
                >
                    requires NotSameAs<T, void>
                friend class MultiArray;

                using Array = OriginType<A>;
                using ArrayDummyVectorType = typename Array::DummyVectorType;

            public:
                using ValueType = typename Array::ValueType;
                using ContainerType = typename Array::ContainerType;
                using ShapeType = OriginType<S>;
                using VectorType = typename ShapeType::VectorType;
                using VectorViewType = typename ShapeType::VectorViewType;
                using DummyVectorType = std::vector<DummyIndex>;
                using DummyVectorViewType = std::span<const DummyIndex>;

                using IterType = MultiArrayViewIterator<MultiArrayView>;
                using ConstIterType = MultiArrayViewConstIterator<MultiArrayView>;
                using ReverseIterType = MultiArrayViewReverseIterator<MultiArrayView>;
                using ConstReverseIterType = MultiArrayViewConstReverseIterator<MultiArrayView>;

                static_assert(multi_array::ShapeType<ShapeType> && ShapeType::dim == dynamic_dimension);

            private:
                constexpr MultiArrayView(const Array& array, ArgRRefType<ArrayDummyVectorType> vector)
                    : _vector(move<ArrayDummyVectorType>(vector)), _array(array)
                {
                    if constexpr (Array::dim == dynamic_dimension)
                    {
                        if (_vector.size() != _array->dimension())
                        {
                            throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", this->dimension(), _vector.size()) };
                        }
                    }
                    std::vector<usize> shape;
                    for (usize i{ 0_uz }; i != _array->dimension(); ++i)
                    {
                        assert(_vector[i].is_single_index() || _vector[i].is_range_full());
                        if (_vector[i].is_range_full())
                        {
                            shape.push_back(_array->shape().shape()[i]);
                            _map_dimension.push_back(i);
                        }
                    }
                    _shape = DynShape{ std::move(shape) };
                }

            public:
                constexpr MultiArrayView(const MultiArrayView& ano) = default;
                constexpr MultiArrayView(MultiArrayView&& ano) noexcept = default;
                constexpr MultiArrayView& operator=(const MultiArrayView& rhs) = default;
                constexpr MultiArrayView& operator=(MultiArrayView&& rhs) noexcept = default;
                constexpr ~MultiArrayView(void) noexcept = default;

            public:
                inline constexpr LRefType<ContainerType> raw(void) noexcept
                {
                    return _array->raw();
                }

                inline constexpr CLRefType<ContainerType> raw(void) const noexcept
                {
                    return _array->raw();
                }

                template<typename = void>
                    requires requires (ContainerType& container) { { container.data() } -> DecaySameAs<PtrType<ValueType>>; }
                inline constexpr const PtrType<ValueType> data(void) noexcept
                {
                    return raw().data();
                }

                template<typename = void>
                    requires requires (const ContainerType& container) { { container.data() } -> DecaySameAs<CPtrType<ValueType>>; }
                inline constexpr const CPtrType<ValueType> data(void) const noexcept
                {
                    return raw().data();
                }

            public:
                inline constexpr LRefType<ValueType> get(const usize index)
                {
                    return _array->get(actual_index(index));
                }

                inline constexpr CLRefType<ValueType> get(const usize index) const
                {
                    return _array->get(actual_index(index));
                }

                inline constexpr LRefType<ValueType> get(ArgCLRefType<VectorViewType> vector)
                {
                    return _array->get(actual_index(vector));
                }

                inline constexpr CLRefType<ValueType> get(ArgCLRefType<VectorViewType> vector) const
                {
                    return _array->get(actual_index(vector));
                }

                inline constexpr DynRefArray<ValueType> get(const RangeFull _) const
                {
                    DynRefArray<ValueType> ret;
                    ret.reserve(_shape.size());
                    auto vector = _shape.zero();
                    do
                    {
                        ret.push_back(get(vector));
                    } while (_shape.next_vector(vector));
                    return ret;
                }

                inline constexpr DynRefArray<ValueType> get(const DummyVectorViewType dummy_vector) const
                {
                    dummy_index::DummyAccessEnumerator<ShapeType> iter{ shape(), dummy_vector };
                    DynRefArray<ValueType> ret;
                    ret.reserve(iter.size());
                    while (iter)
                    {
                        const auto vector = *iter;
                        ret.push_back(get(vector));
                        ++iter;
                    }
                    return ret;
                }
                
                template<usize vec_dim, usize to_dim>
                    requires (to_dim != 0_uz)
                inline constexpr decltype(auto) get(const map_index::MapVector<vec_dim, to_dim>& vector) const
                {
                    if (vec_dim != dimension())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", shape.dimension(), vec_dim) };
                    }

                    const auto map_dimension = map_to_dimension(vector);
                    const auto to_shape = map_to_shape(shape, map_dimension);
                    auto base_vector = base_map_to_vector(shape, vector);

                    DynRefArray<DynRefArray<ValueType>> ret;
                    ret.reserve(to_shape.size());
                    auto map_vector = to_shape.zero();
                    do
                    {
                        for (usize i{ 0_uz }; i != to_dim; ++i)
                        {
                            base_vector[map_dimension[i]] = DummyIndex{ map_vector[i] };
                        }
                        ret.push_back(get(base_vector));
                    } while (to_shape.next_vector(map_vector));

                    return std::make_pair(std::move(to_shape), std::move(ret));
                }

                template<usize vec_dim, usize to_dim>
                    requires (to_dim != 0_uz)
                inline constexpr decltype(auto) get(map_index::MapVector<vec_dim, to_dim>&& vector) const
                {
                    if (vec_dim != dimension())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", shape.dimension(), vec_dim) };
                    }

                    const auto map_dimension = map_to_dimension(vector);
                    const auto to_shape = map_to_shape(shape, map_dimension);
                    auto base_vector = base_map_to_vector(shape, std::move(vector));

                    DynRefArray<DynRefArray<ValueType>> ret;
                    ret.reserve(to_shape.size());
                    auto map_vector = to_shape.zero();
                    do
                    {
                        for (usize i{ 0_uz }; i != to_dim; ++i)
                        {
                            base_vector[map_dimension[i]] = DummyIndex{ map_vector[i] };
                        }
                        ret.push_back(get(base_vector));
                    } while (to_shape.next_vector(map_vector));

                    return std::make_pair(std::move(to_shape), std::move(ret));
                }

            public:
                // todo: use operator[...] to replace operator(...) in C++23

                template<typename... Args>
                    requires is_normal_vector<Args...>
                inline constexpr LRefType<ValueType> operator()(Args&&... args)
                {
                    return get(normal_vector(shape(), std::forward<Args>(args)...));
                }
                
                template<typename... Args>
                    requires is_normal_vector<Args...>
                inline constexpr CLRefType<ValueType> operator()(Args&&... args) const
                {
                    return get(normal_vector(shape(), std::forward<Args>(args)...));
                }

                template<typename... Args>
                    requires is_dummy_vector<Args...> && !is_normal_vector<Args...>
                inline constexpr DynRefArray<ValueType> operator()(Args&&... args) const
                {
                    return get(DummyVectorType{ DummyIndex{ std::forward<Args>(args) }... });
                }

                template<typename... Args>
                    requires is_map_vector<Args...> && !is_dummy_vector<Args...> && !is_normal_vector<Args...>
                inline constexpr decltype(auto) operator()(Args&&... args) const
                {
                    return get(map_vector(std::forward<Args>(args)...));
                }

            public:
                inline constexpr LRefType<ValueType> operator[](const VectorViewType vector)
                {
                    return get(vector);
                }

                inline constexpr CLRefType<ValueType> operator[](const VectorViewType vector) const
                {
                    return get(vector);
                }

                inline constexpr LRefType<ValueType> operator[](std::initializer_list<usize> vector)
                {
                    return get(VectorType{ std::move(vector) });
                }

                inline constexpr CLRefType<ValueType> operator[](std::initializer_list<usize> vector) const
                {
                    return get(VectorType{ std::move(vector) });
                }

                inline constexpr DynRefArray<ValueType> operator[](const DummyVectorViewType vector) const
                {
                    return get(vector);
                }

                inline constexpr DynRefArray<ValueType> operator[](std::initializer_list<DummyIndex> vector) const
                {
                    return get(DummyVectorType{ vector });
                }

                template<usize vec_dim, usize to_dim>
                    requires (to_dim != 0_uz)
                inline constexpr decltype(auto) operator[](const map_index::MapVector<vec_dim, to_dim>& vector) const
                {
                    return get(vector);
                }

            public:
                template<typename... Args>
                    requires (VariableTypeList<Args>::length != 0_uz) && is_view_vector<Args...>
                inline constexpr MultiArrayView view(Args&&... args) const
                {
                    static constexpr const usize dim = VariableTypeList<Args>::length;
                    if (dim > _shape.dimension())
                    {
                        throw OSPFException{ OSPFErrCode::ApplicationError, std::format("dimension should be {}, not {}", _shape.dimension(), dim) };
                    }
                    auto vector = _vector;
                    view_vector<0_uz, dim>(vector, std::forward<Args>(args)...);
                    return MultiArrayView(*_array, std::move(vector));
                }

            public:
                inline constexpr IterType begin(void) noexcept
                {
                    return IterType{ _shape.zero(), *this};
                }

                inline constexpr ConstIterType begin(void) const noexcept
                {
                    return ConstIterType{ _shape.zero(), *this };
                }

                inline constexpr ConstIterType cbegin(void) const noexcept
                {
                    return ConstIterType{ _shape.zero(), *this };
                }

                inline constexpr IterType end(void) noexcept
                {
                    return IterType{ *this };
                }

                inline constexpr ConstIterType end(void) const noexcept
                {
                    return ConstIterType{ *this };
                }

                inline constexpr ConstIterType cend(void) const noexcept
                {
                    return ConstIterType{ *this };
                }

                inline constexpr ReverseIterType rbegin(void) noexcept
                {
                    //const auto vector = _shape.shape();
                    //return ReverseIterType{ DynShape::VectorType{ vector.begin(), vector.end() }, *this };
                    return ReverseIterType{ _shape.shape(), *this };
                }

                inline constexpr ConstReverseIterType rbegin(void) const noexcept
                {
                    //const auto vector = _shape.shape();
                    //return ConstReverseIterType{ DynShape::VectorType{ vector.begin(), vector.end() }, *this };
                    return ConstReverseIterType{ _shape.shape(), *this };
                }

                inline constexpr ConstReverseIterType crbegin(void) const noexcept
                {
                    //const auto vector = _shape.shape();
                    //return ConstReverseIterType{ DynShape::VectorType{ vector.begin(), vector.end() }, *this };
                    return ConstReverseIterType{ _shape.shape(), *this };
                }

                inline constexpr ReverseIterType rend(void) noexcept
                {
                    return ReverseIterType{ *this };
                }

                inline constexpr ConstReverseIterType rend(void) const noexcept
                {
                    return ConstReverseIterType{ *this };
                }

                inline constexpr ConstReverseIterType crend(void) const noexcept
                {
                    return ConstReverseIterType{ *this };
                }

            public:
                inline constexpr CLRefType<ShapeType> shape(void) const noexcept
                {
                    return _shape;
                }

                inline constexpr const usize dimension(void) const noexcept
                {
                    return shape().dimension();
                }

                inline constexpr const usize size(void) const noexcept
                {
                    return shape().size();
                }

                inline constexpr const usize max_size(void) const noexcept
                {
                    return raw().max_size();
                }

            public:
                inline constexpr const bool operator==(const MultiArrayView& rhs) const noexcept
                {
                    return _shape == rhs._shape
                        && _map_dimension == rhs._map_dimension
                        && _vector == rhs._vector
                        && _array == rhs._array;
                }

                inline constexpr const bool operator!=(const MultiArrayView& rhs) const noexcept
                {
                    return _shape != rhs._shape
                        || _map_dimension != rhs._map_dimension
                        || _vector != rhs._vector
                        || _array != rhs._array;
                }

            private:
                inline constexpr const usize actual_index(ArgCLRefType<VectorViewType> this_vector) const
                {
                    auto ret = _array.shape().zero();
                    assert(this_vector.size() == _map_dimension);
                    for (usize i{ 0_uz }, j{ 0_uz }; _array.dimension(); ++i)
                    {
                        if (i == _map_dimension[j])
                        {
                            assert(_vector[i].is_range_full());
                            ret[i] = this_vector[j];
                            ++j;
                        }
                        else
                        {
                            assert(_vector[i].is_single_index());
                            auto index = _vector[i].single_index();
                            assert(index.has_value());
                            std::visit([this, &ret, i](const auto index)
                            {
                                using IndexType = OriginType<decltype(index)>;
                                if constexpr (DecaySameAs<decltype(index), IndexType>)
                                {
                                    ret[i] = index;
                                }
                                else if constexpr (DecaySameAs<decltype(index), IndexType>)
                                {
                                    ret[i] = this->_array.shape().actual_index(i, index);
                                }
                            }, *index);
                        }
                        return _array.shape().index(ret);
                    }
                }

                inline constexpr const usize actual_index(const usize index) const
                {
                    return actual_index(index);
                }

            public:
                template<usize i, usize dim, typename T, typename... Args>
                inline static constexpr void view_vector(ArrayDummyVectorType& vector, T&& arg, Args&&... args)
                {
                    if constexpr (i == dim)
                    {
                        return;
                    }
                    else
                    {
                        if constexpr (DecaySameAs<T, usize> || std::unsigned_integral<OriginType<T>>)
                        {
                            vector[_map_dimension[i]] = DummyIndex{ static_cast<usize>(arg) };
                        }
                        else if constexpr (DecaySameAs<T, isize> || std::signed_integral<OriginType<T>>)
                        {
                            vector[_map_dimension[i]] = DummyIndex{ static_cast<isize>(arg) };
                        }
                        else if constexpr (DecaySameAs<T, RangeFull>)
                        {
                            vector[_map_dimension[i]] = DummyIndex{ __ };
                        }
                        view_vector<i + 1_uz, dim>(vector, std::forward<Args>(args)...);
                    }
                }

            private:
                DynShape _shape;
                std::vector<usize> _map_dimension;
                ArrayDummyVectorType _vector;
                Ref<Array> _array;
            };
        };
    };
};

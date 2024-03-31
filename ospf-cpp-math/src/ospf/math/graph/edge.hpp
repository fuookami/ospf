#pragma once

#include <ospf/math/graph/node.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace graph
        {
            template<typename T, typename N>
            class Edge
            {
            public:
                using ValueType = OriginType<T>;
                using NodeType = OriginType<N>;

            public:
                constexpr Edge(ArgCLRefType<ValueType> data, const NodeType& from, const NodeType& to)
                    : _data(data), _from(from), _to(to) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Edge(ArgRRefType<ValueType> data, const NodeType& from, const NodeType& to)
                    : _data(move<ValueType>(data)), _from(from), _to(to) {}

            public:
                constexpr Edge(const Edge& ano) = default;
                constexpr Edge(Edge&& ano) noexcept = default;
                constexpr Edge& operator=(const Edge& rhs) = default;
                constexpr Edge& operator=(Edge&& rhs) noexcept = default;
                constexpr ~Edge(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return _data;
                }

                inline constexpr CLRefType<ValueType> data(void) const noexcept
                {
                    return _data;
                }

            public:
                inline const NodeType& from(void) const noexcept
                {
                    return _from;
                }

                inline const NodeType& to(void) const noexcept
                {
                    return _to;
                }

            private:
                Ref<NodeType> _from;
                Ref<NodeType> _to;

                ValueType _data;
            };

            template<typename N>
            class Edge<void, N>
            {
            public:
                using ValueType = void;
                using NodeType = OriginType<N>;

            public:
                constexpr Edge(const NodeType& from, const NodeType& to)
                    : _from(from), _to(to) {}
                constexpr Edge(const Edge& ano) = default;
                constexpr Edge(Edge&& ano) noexcept = default;
                constexpr Edge& operator=(const Edge& rhs) = default;
                constexpr Edge& operator=(Edge&& rhs) noexcept = default;
                constexpr ~Edge(void) noexcept = default;

            public:
                inline const NodeType& from(void) const noexcept
                {
                    return _from;
                }

                inline const NodeType& to(void) const noexcept
                {
                    return _to;
                }

            private:
                Ref<NodeType> _from;
                Ref<NodeType> _to;
            };
        };
    };
};

namespace ospf
{
    template<typename T, typename N>
        requires WithTag<N>
    struct TagValue<graph::Edge<T, N>>
    {
        using Type = typename TagValue<N>::Type;

        inline constexpr RetType<Type> operator()(ArgCLRefType<graph::Edge<T, N>> edge) const noexcept
        {
            static const TagValue<N> extractor{};
            return extractor(edge.from());
        }
    };
};

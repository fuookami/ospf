#pragma once

#include <ospf/data_structure/reference_array.hpp>
#include <ospf/data_structure/tagged_map.hpp>
#include <ospf/math/graph/edge.hpp>
#include <ospf/memory/pool.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace graph
        {
            template<typename N, typename E>
                requires DecaySameAs<N, typename E::NodeType>
            class Graph
            {
            public:
                using NodeType = OriginType<N>;
                using NodeValueType = typename NodeType::ValueType;
                using EdgeType = OriginType<E>;
                using EdgeValueType = typename EdgeType::ValueType;

            public:
                constexpr Graph(void) = default;
                constexpr Graph(const Graph& ano) = delete;
                constexpr Graph(Graph&& ano) noexcept = default;
                constexpr Graph& operator=(const Graph& rhs) = delete;
                constexpr Graph& operator=(Graph&& rhs) noexcept = delete;
                constexpr ~Graph(void) noexcept
                {
                    _edges.clear();
                    _nodes.clear();
                    std::thread([](ObjectPool<NodeType> nodes, ObjectPool<EdgeType> edges)
                        {
                            // nothing to do, just send the pools to sub thread to destroy
                        }, std::move(_node_pool), std::move(_edge_pool)).join();
                }

            public:
                inline const auto& nodes(void) const noexcept
                {
                    return _nodes;
                }

                inline const auto& edges(void) const noexcept
                {
                    return _edges;
                }

            public:
                template<typename... Args>
                inline const bool insert_node(Args&&... args) noexcept
                {
                    NodeValueType node_value{ std::forward<Args>(args)... };
                    if constexpr (WithTag<NodeValueType>)
                    {
                        static const TagValue<NodeValueType> extractor{};
                        if (_nodes.contains(extractor.value(node_value)))
                        {
                            return false;
                        }
                        auto new_node = _node_pool.make_ptr(std::move(node_value));
                        _nodes.insert(new_node);
                    }
                    else
                    {
                        if (_nodes.contains(node_value))
                        {
                            return false;
                        }
                        auto new_node = _node_pool.make_ptr(std::move(node_value));
                        _nodes.insert(new_node);
                    }
                    return true;
                }

                inline std::optional<Ref<NodeType>> get_node(ArgCLRefType<typename TagValue<NodeType>::Type> key) const noexcept
                {
                    auto it = _nodes.find(key);
                    if (it == _nodes.cend())
                    {
                        return std::nullopt;
                    }
                    return *it;
                }

            public:
                template<typename = void>
                    requires DecayNotSameAs<void>
                inline const bool insert_edge(const Ref<NodeType> from, const Ref<NodeType> to) noexcept
                {
                    if (connected(from, to))
                    {
                        return false;
                    }
                    else
                    {
                        auto new_edge = _edge_pool.make_ptr(from, to);
                        _edges.insert(std::move(new_edge));
                        return true;
                    }
                }

                template<typename... Args>
                    requires std::convertible_to<EdgeValueType, Args...>
                inline const bool insert_edge(const Ref<NodeType> from, const Ref<NodeType> to, Args&&... args) noexcept
                {
                    if (connected(from, to))
                    {
                        return false;
                    }
                    else
                    {
                        auto new_edge = _edge_pool.make_ptr(EdgeValueType{ std::forward<Args>(args)... }, from, to);
                        _edges.insert(std::move(new_edge));
                        return true;
                    }
                }

            public:
                inline DynRefArray<EdgeType> get_edges(const Ref<NodeType> from) const noexcept
                {
                    const auto range = _edges.equal_range(from);
                    return DynRefArray<EdgeType>{ range.first, range.second };
                }

                inline DynRefArray<EdgeType> get_edges_to(const Ref<NodeType> to) const noexcept
                {
                    DynRefArray<EdgeType> ret;
                    for (const auto edge : _edges)
                    {
                        if (edge->to() == to)
                        {
                            ret.push_back(edge);
                        }
                    }
                    return ret;
                }

                inline const bool connected(const Ref<NodeType> from, const Ref<NodeType> to) const noexcept
                {
                    const auto range = _edges.equal_range(from);
                    for (auto it{ range.first }; it != range.second; ++it)
                    {
                        if ((**it).to() == to)
                        {
                            return true;
                        }
                    }
                    return false;
                }

            private:
                ObjectPool<NodeType> _node_pool;
                ObjectPool<EdgeType> _edge_pool;
                TaggedMap<Ref<NodeType>> _nodes;
                TaggedMultiMap<Ref<EdgeType>> _edges;
            };
        };
    };
};

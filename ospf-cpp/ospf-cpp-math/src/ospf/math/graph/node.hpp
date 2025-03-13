#pragma once

#include <ospf/concepts/with_default.hpp>
#include <ospf/concepts/with_tag.hpp>
#include <ospf/memory/pointer.hpp>
#include <ospf/memory/reference.hpp>
#include <ospf/functional/value_or_reference.hpp>
#include <ospf/functional/pointer_or_reference.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace graph
        {
            template<typename T>
            class Node
            {
            public:
                using ValueType = OriginType<T>;

            public:
                template<typename = void>
                    requires WithDefault<ValueType>
                constexpr Node(void)
                    : _data(DefaultValue<ValueType>::value()) {}

                constexpr Node(ArgCLRefType<ValueType> data)
                    : _data(data) {}

                template<typename = void>
                    requires ReferenceFaster<ValueType> && std::movable<ValueType>
                constexpr Node(ArgRRefType<ValueType> data)
                    : _data(move<ValueType>(data)) {}

                template<typename... Args>
                    requires std::constructible_from<ValueType, Args...>
                constexpr Node(Args&&... args)
                    : _data(std::forward<Args>(args)...) {}

            public:
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return _data;
                }

                inline constexpr ArgCLRefType<ValueType> data(void) const noexcept
                {
                    return _data;
                }

            private:
                ValueType _data;
            };

            template<>
            class Node<void>
            {
            public:
                constexpr Node(void) = default;
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr const std::nullopt_t data(void) const noexcept
                {
                    return std::nullopt;
                }
            };

            template<typename T, ospf::PointerCategory cat>
            class Node<ospf::pointer::Ptr<T, cat>>
            {
            public:
                using ValueType = OriginType<T>;
                using PointerType = ospf::pointer::Ptr<T, cat>>;

            public:
                template<typename = void>
                    requires WithDefault<PointerType>
                constexpr Node(void)
                    : _ptr(DefaultValue<PointerType>::value()) {}

                constexpr Node(ArgCLRefType<PointerType> ptr)
                    : _ptr(ptr) {}

                template<typename = void>
                    requires ReferenceFaster<PointerType> && std::movable<PointerType>
                constexpr Node(ArgRRefType<PointerType> ptr)
                    : _ptr(move<PointerType>(ptr)) {}

            public:
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return *_ptr;
                }

                inline constexpr ArgCLRefType<ValueType> data(void) const noexcept
                {
                    return *_ptr;
                }

            private:
                PointerType _ptr;
            };

            template<typename T, ospf::ReferenceCategory cat>
            class Node<ospf::reference::Ref<T, cat>>
            {
            public:
                using ValueType = OriginType<T>;
                using ReferenceType = ospf::reference::Ref<T, cat>;

            public:
                constexpr Node(ArgCLRefType<ReferenceType> ref)
                    : _ref(ref) {}

                template<typename = void>
                    requires ReferenceFaster<ReferenceType> && std::movable<ReferenceType>
                constexpr Node(ArgRRefType<ReferenceType> ref)
                    : _ref(move<ReferenceType>(ref)) {}

            public:
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return *_ref;
                }

                inline constexpr ArgCLRefType<ValueType> data(void) const noexcept
                {
                    return *_ref;
                }

            private:
                ReferenceType _ref;
            };

            template<typename T, ospf::ReferenceCategory cat, ospf::CopyOnWrite cow>
            class Node<ospf::ValOrRef<T, cat, cow>>
            {
            public:
                using ValueType = OriginType<T>;
                using ValOrRefType = ospf::ValOrRef<ValueType, cat, cow>;

            public:
                constexpr Node(ArgCLRefType<ValOrRefType> value)
                    : _value(value) {}

                template<typename = void>
                    requires ReferenceFaster<ValOrRefType> && std::movable<ValOrRefType>
                constexpr Node(ArgRRefType<ValOrRefType> value)
                    : _value(move<ValOrRefType>(value)) {}

            public:
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return *_value;
                }

                inline constexpr ArgCLRefType<ValueType> data(void) const noexcept
                {
                    return *_value;
                }

            private:
                ValOrRefType _value;
            };

            template<typename T, ospf::PointerCategory pcat, ospf::ReferenceCategory rcat>
            class Node<ospf::PtrOrRef<T, pcat, rcat>>
            {
            public:
                using ValueType = OriginType<T>;
                using PtrOrRefType = ospf::PtrOrRef<ValueType, pcat, rcat>;

            public:
                constexpr Node(ArgCLRefType<PtrOrRefType> value)
                    : _value(value) {}

                template<typename = void>
                    requires ReferenceFaster<PtrOrRefType> && std::movable<PtrOrRefType>
                constexpr Node(ArgRRefType<PtrOrRefType> value)
                    : _value(move<PtrOrRefType>(value)) {}

            public:
                constexpr Node(const Node& ano) = default;
                constexpr Node(Node&& ano) noexcept = default;
                constexpr Node& operator=(const Node& rhs) = default;
                constexpr Node& operator=(Node&& rhs) noexcept = default;
                constexpr ~Node(void) noexcept = default;

            public:
                inline constexpr LRefType<ValueType> data(void) noexcept
                {
                    return *_value;
                }

                inline constexpr ArgCLRefType<ValueType> data(void) const noexcept
                {
                    return *_value;
                }

            private:
                PtrOrRefType _value;
            };
        };
    };
};

namespace ospf
{
    template<typename T>
    struct TagValue<Node<T>>
    {
        using Type = std::conditional_t<WithTag<T>, typename TagValue<T>::Type, T>;

        inline constexpr RetType<Type> operator()(ArgCLRefType<Node<T>> node) const noexcept
        {
            if constexpr (WithTag<T>)
            {
                static const TagValue<T> extractor{};
                return extractor(node.data());
            }
            else
            {
                return node.data();
            }
        }
    };
};

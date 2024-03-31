#include <ospf/meta_programming/name_transfer.hpp>

namespace ospf::meta_programming::name_transfer
{
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::KebabCase, char>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::CamelCase, char>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::PascalCase, char>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, char>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::KebabCase, wchar>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::CamelCase, wchar>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::PascalCase, wchar>;
    template class NameTransferImpl<NamingSystem::SnakeCase, NamingSystem::UpperSnakeCase, wchar>;

    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::SnakeCase, char>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::CamelCase, char>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::PascalCase, char>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::UpperSnakeCase, char>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::SnakeCase, wchar>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::CamelCase, wchar>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::PascalCase, wchar>;
    template class NameTransferImpl<NamingSystem::KebabCase, NamingSystem::UpperSnakeCase, wchar>;

    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::SnakeCase, char>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::KebabCase, char>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::PascalCase, char>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::UpperSnakeCase, char>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::SnakeCase, wchar>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::KebabCase, wchar>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::PascalCase, wchar>;
    template class NameTransferImpl<NamingSystem::CamelCase, NamingSystem::UpperSnakeCase, wchar>;

    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::SnakeCase, char>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::KebabCase, char>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::CamelCase, char>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::UpperSnakeCase, char>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::SnakeCase, wchar>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::KebabCase, wchar>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::CamelCase, wchar>;
    template class NameTransferImpl<NamingSystem::PascalCase, NamingSystem::UpperSnakeCase, wchar>;

    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::SnakeCase, char>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::KebabCase, char>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::CamelCase, char>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::PascalCase, char>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::SnakeCase, wchar>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::KebabCase, wchar>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::CamelCase, wchar>;
    template class NameTransferImpl<NamingSystem::UpperSnakeCase, NamingSystem::PascalCase, wchar>;
};

#include <ospf/string/hasher.hpp>

namespace ospf::string
{
    template struct StringHasher<char>;
    template struct StringHasher<wchar>;
};

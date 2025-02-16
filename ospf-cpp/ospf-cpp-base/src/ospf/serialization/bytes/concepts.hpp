#pragma once

#include <ospf/concepts/base.hpp>
#include <ospf/basic_definition.hpp>
#include <iterator>

namespace ospf
{
    inline namespace serialization
    {
        namespace bytes
        {
            template<typename It>
            concept FromValueIter = std::input_iterator<It> 
                && requires (const It& it)
            {
                { *it } -> DecaySameAs<ubyte>;
            };

            template<typename It>
            concept ToValueIter = std::output_iterator<It, ubyte>;

            using NameTransfer = std::function<const std::string_view(const std::string_view)>;
        };
    };
};

#pragma once

#include <ospf/system_info.hpp>
#include <random>
#include <boost/random.hpp>

namespace ospf
{
    inline namespace random
    {
        template<typename T, usize digits = address_length * sizeof(T)>
        using RandomGenerator = boost::random::independent_bits_engine<std::mt19937_64, digits, T>;

        template<typename T, usize digits = address_length * sizeof(T)>
        inline RandomGenerator<T, digits> random_generator(void) noexcept
        {
            static std::random_device device;
            std::seed_seq seq{ device(), device(), device(), device() };

            RandomGenerator<T, digits> gen;
            gen.seed(seq);
            return gen;
        }
    };
};

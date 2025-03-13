#pragma once

#include <ospf/math/geometry/point.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            // projection from dimension 3 to 2
            enum class ProjectionPlane : u8
            {
                Front,      // XOY
                Side,       // ZOY
                Bottom      // ZOX
            };

            template<ProjectionPlane plane>
            struct ProjectionPlaneTrait;

            template<>
            struct ProjectionPlaneTrait<ProjectionPlane::Front>
            {

            };

            template<>
            struct ProjectionPlaneTrait<ProjectionPlane::Side>
            {

            };

            template<>
            struct ProjectionPlaneTrait<ProjectionPlane::Bottom>
            {

            };
        };
    };
};

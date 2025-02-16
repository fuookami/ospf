#pragma once

#include <ospf/math/algebra/operator/comparison/less.hpp>
#include <ospf/math/algebra/operator/comparison/less_equal.hpp>
#include <ospf/math/geometry/circle.hpp>
#include <ospf/math/geometry/edge.hpp>
#include <ospf/math/geometry/point.hpp>
#include <ospf/math/geometry/triangle.hpp>

namespace ospf
{
    inline namespace math
    {
        inline namespace geometry
        {
            namespace triangulation
            {
                template<RealNumber T = f64>
                class Delaunay
                {
                public:
                    using ValueType = OriginType<T>;
                    using PointType = Point2<ValueType>;
                    using EdgeType = Edge2<ValueType>;
                    using TriangleType = Triangle2<ValueType>;
                    using CircleType = Circle2<ValueType>;

                public:
                    constexpr Delaunay(void) = default;
                    constexpr Delaunay(const Delaunay& ano) = default;
                    constexpr Delaunay(Delaunay&& ano) noexcept = default;
                    constexpr Delaunay& operator=(const Delaunay& rhs) = default;
                    constexpr Delaunay& operator=(Delaunay&& rhs) noexcept = default;
                    constexpr ~Delaunay(void) noexcept = default;

                public:
                    // Bowyer-Watson algorithm
                    // C++ implementation of http://paulbourke.net/papers/triangulate
                    inline std::vector<TriangleType> operator()(std::vector<PointType> points) const noexcept
                    {
                        if (points.size() < 3_uz)
                        {
                            return std::vector<TriangleType>{};
                        }
                        std::ranges::sort(points, [](ArgCLRefType<PointType> lhs, ArgCLRefType<PointType> rhs)
                            {
                                static const Less<ValueType> op{};
                                if (op(lhs.x(), rhs.x()))
                                {
                                    return true;
                                }
                                else if (op(rhs.x(), lhs.x()))
                                {
                                    return false;
                                }
                                else
                                {
                                    return op(lhs.y(), rhs.y());
                                }
                            });

                        const auto super_triangle = get_super_triangle(points);
                        std::vector<TriangleType> triangles{ super_triangle };
                        triangles.reserve(points.size());
                        std::vector<TriangleType> undetermined_triangles{ super_triangle };
                        undetermined_triangles.reserve(points.size());

                        for (ArgCLRefType<PointType> point : points)
                        {
                            std::vector<EdgeType> edges;
                            edges.reserve(undetermined_triangles.size() * 3_uz);
                            std::vector<TriangleType> this_triangles;
                            this_triangles.reserve(undetermined_triangles.size());

                            for (const TriangleType& triangle : undetermined_triangles)
                            {
                                static const LessEqual<ValueType> op{};

                                // check if the point is inside the triangle circum circle
                                const CircleType circum_circle{ CircleType::circum_circle_of(triangle) };
                                const auto dist = **sqrt(sqr(circum_circle.x() - point.x()) + sqr(circum_circle.y() - point.y()));
                                if (triangle.illegal() || op(dist, circum_circle.radius()))
                                {
                                    edges.push_back(triangle.e1());
                                    edges.push_back(triangle.e2());
                                    edges.push_back(triangle.e3());
                                }
                                else if (op(circum_circle.x() + circum_circle.radius(), point.x()))
                                {
                                    triangles.push_back(triangle);
                                }
                                else
                                {
                                    this_triangles.push_back(triangle);
                                }
                            }

                            delete_duplicate_edges(edges);
                            update_triangles(this_triangles, point, edges);
                            std::swap(undetermined_triangles, this_triangles);
                        }

                        remove_origin_super_triangle(triangles, super_triangle, undetermined_triangles);
                        triangles.shrink_to_fit();
                        return triangles;
                    }


                private:
                    inline static void delete_duplicate_edges(std::vector<EdgeType>& edges) noexcept
                    {
                        std::vector<bool> duplication;
                        duplication.resize(edges.size(), false);
                        for (usize i{ 0_uz }; i != edges.size(); ++i)
                        {
                            if (duplication[i])
                            {
                                continue;
                            }

                            for (usize j{ i + 1_uz }; j != edges.size(); ++j)
                            {
                                if (edges[i] == edges[j])
                                {
                                    duplication[i] = true;
                                    duplication[j] = true;
                                }
                            }
                        }
                        std::vector<EdgeType> new_edges;
                        new_edges.reserve(edges.size());
                        for (usize i{ 0_uz }; i != edges.size(); ++i)
                        {
                            if (!duplication[i])
                            {
                                new_edges.push_back(edges[i]);
                            }
                        }
                        new_edges.shrink_to_fit();
                        std::swap(edges, new_edges);
                    }

                    inline static void update_triangles(std::vector<TriangleType>& triangles, ArgCLRefType<PointType> point, const std::vector<EdgeType>& edges) noexcept
                    {
                        for (ArgCLRefType<EdgeType> edge : edges)
                        {
                            if constexpr (CopyFaster<PointType>)
                            {
                                triangles.push_back(TriangleType{ edge.from(), edge.to(), point });
                            }
                            else
                            {
                                triangles.push_back(TriangleType
                                    {
                                        COW<PointType>::ref(edge.from()),
                                        COW<PointType>::ref(edge.to()),
                                        COW<PointType>::ref(point)
                                    });
                            }
                        }
                    }

                    inline static void remove_origin_super_triangle(std::vector<TriangleType>& triangles, const TriangleType& super_triangle, const std::vector<TriangleType>& undetermined_triangles) noexcept
                    {
                        const auto is_super_triangle = [&super_triangle](const TriangleType& triangle)
                        {
                            return triangle.illegal()
                                || (triangle.p1() == super_triangle.p1() || triangle.p2() == super_triangle.p1() || triangle.p3() == super_triangle.p1())
                                || (triangle.p1() == super_triangle.p2() || triangle.p2() == super_triangle.p2() || triangle.p3() == super_triangle.p2())
                                || (triangle.p1() == super_triangle.p3() || triangle.p2() == super_triangle.p3() || triangle.p3() == super_triangle.p3());
                        };
                        triangles.erase(
                            std::remove_if(triangles.begin(), triangles.end(), is_super_triangle),
                            triangles.end()
                        );
                        std::copy_if(undetermined_triangles.begin(), undetermined_triangles.end(), std::back_inserter(triangles),
                            [&is_super_triangle](const TriangleType& triangle)
                            {
                                return !is_super_triangle(triangle);
                            });
                    }

                    inline static constexpr RetType<TriangleType> get_super_triangle(const std::vector<PointType>& points) noexcept
                    {
                        static const Less<ValueType> op{};
                        static const ValueType two{ RealNumberTrait<ValueType>::two() };

                        const auto [min_x_it, max_x_it] = std::ranges::minmax_element(points, 
                            [](ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs)
                            {
                                return op(lhs, rhs);
                            },
                            [](ArgCLRefType<PointType> ele)
                            {
                                return ele.x();
                            });
                        const auto min_x = min_x_it->x();
                        const auto max_x = max_x_it->x();
                        const auto dx = max_x - min_x;
                        const auto mid_x = (max_x + min_x) / two;

                        const auto [min_y_it, max_y_it] = std::ranges::minmax_element(points, 
                            [](ArgCLRefType<ValueType> lhs, ArgCLRefType<ValueType> rhs)
                            {
                                return op(lhs, rhs);
                            },
                            [](ArgCLRefType<PointType> ele)
                            {
                                return ele.y();
                            });
                        const auto min_y = min_y_it->y();
                        const auto max_y = max_y_it->y();
                        const auto dy = max_y - min_y;
                        const auto mid_y = (max_y + min_y) / two;

                        const auto dmax = (std::max)(dx, dy, op);
                        return TriangleType
                        {
                            PointType{ mid_x - two * dmax, mid_y - dmax },
                            PointType{ mid_x, mid_y + two * dmax },
                            PointType{ mid_x + two * dmax, mid_y - dmax }
                        };
                    }
                };

                extern template class Delaunay<f64>;
            };

            template<RealNumber T = f64>
            inline constexpr std::vector<Triangle2<T>> triangulate(std::vector<Point2<T>> points) noexcept
            {
                static const triangulation::Delaunay<T> algorithm{};
                return algorithm(std::move(points));
            }

            template<RealNumber T = f64>
            inline constexpr std::vector<Triangle3<T>> triangulate(const std::vector<Point3<T>>& points) noexcept
            {
                const auto get_point = [&points](ArgCLRefType<Point2<T>> point2) -> ArgCLRefType<Point3<T>>
                {
                    static const Equal<T> op{};

                    for (ArgCLRefType<Point3<T>> point : points)
                    {
                        if (op(point.x(), point2.x()) && op(point.y(), point2.y()))
                        {
                            return point;
                        }
                    }

                    assert(false);
                    return points.front();
                };

                std::vector<Point2<T>> point2s;
                point2s.reserve(points.size());
                for (ArgCLRefType<Point3<T>> point : points)
                {
                    point2s.push_back(Point2<T>
                    {
                        point.x(),
                        point.y()
                    });
                }
                const std::vector<Triangle2<T>> triangle2s = triangulate(std::move(point2s));
                
                std::vector<Triangle3<T>> triangles;
                triangles.reserve(triangle2s.size());
                for (const Triangle2<T>& triangle : triangle2s)
                {
                    if constexpr (CopyFaster<Point3<T>>)
                    {
                        triangles.push_back(Triangle3<T>{
                            get_point(triangle.p1()),
                            get_point(triangle.p2()),
                            get_point(triangle.p3())
                        });
                    }
                    else
                    {
                        triangles.push_back(Triangle3<T>{
                            COW<Point3<T>>::ref(get_point(triangle.p1())),
                            COW<Point3<T>>::ref(get_point(triangle.p2())),
                            COW<Point3<T>>::ref(get_point(triangle.p3()))
                        });
                    }
                }
                return triangles;
            }
        };
    };
};

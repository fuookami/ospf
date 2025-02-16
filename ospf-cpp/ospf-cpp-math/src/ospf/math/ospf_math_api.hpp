#pragma once

#ifndef OSPF_MATH_API
#if defined(_WINDLL) & defined(OSPF_MATH_LIB)
#define OSPF_MATH_API __declspec(dllexport)
#elif defined(_WIN32) & defined(OSPF_DYN_LINK) & (defined(USE_OSPF_MATH_LIB) || defined(USE_OSPF_LIB))
#define OSPF_MATH_API __declspec(dllimport)
#define IMPORT_OSPF_MATH_LIB
#elif defined(USE_OSPF_MATH_LIB) || defined(USE_OSPF_LIB)
#define OSPF_MATH_API
#define IMPORT_OSPF_MATH_LIB
#else
#define OSPF_MATH_API
#endif
#endif

#ifndef IMPORTED_OSPF_MATH_LIB
#ifdef IMPORT_OSPF_MATH_LIB
#define OSPF_LIB_NAME "ospf-cpp-math"
#include <ospf/auto_link.hpp>
#define IMPORTED_OSPF_MATH_LIB
#endif
#endif

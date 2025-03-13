#pragma once

#ifndef OSPF_BASE_API
#if defined(_WINDLL) & defined(OSPF_BASE_LIB)
#define OSPF_BASE_API __declspec(dllexport)
#elif defined(_WIN32) & defined(OSPF_DYN_LINK) & (defined(USE_OSPF_BASE_LIB) || defined(USE_OSPF_LIB))
#define OSPF_BASE_API __declspec(dllimport)
#define IMPORT_OSPF_BASE_LIB
#elif defined(USE_OSPF_BASE_LIB) || defined(USE_OSPF_LIB)
#define OSPF_BASE_API
#define IMPORT_OSPF_BASE_LIB
#else
#define OSPF_BASE_API
#endif
#endif

#ifndef IMPORTED_OSPF_BASE_LIB
#ifdef IMPORT_OSPF_BASE_LIB
#define OSPF_LIB_NAME "ospf-cpp-base"
#include <ospf/auto_link.hpp>
#define IMPORTED_OSPF_BASE_LIB
#endif
#endif

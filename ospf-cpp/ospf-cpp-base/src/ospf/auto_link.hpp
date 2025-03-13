#include <boost/config.hpp>

#ifdef BOOST_MSVC
#ifdef OSPF_DYN_LINK
#define OSPF_LIB_PREFIX ""
#else
#define OSPF_LIB_PREFIX "lib"
#endif

#ifdef _DEBUG
#define OSPF_LIB_SUFFIX "d"
#else
#define OSPF_LIB_SUFFIX ""
#endif

#ifndef BOOST_EMBTC_WIN64
#define OSPF_LIB_EXTENSION ".lib"
#else
#define OSPF_LIB_EXTENSION ".a"
#endif

#ifdef OSPF_LIB_EXTENSION
#if defined(OSPF_LIB_NAME) && defined(OSPF_LIB_PREFIX) && defined(OSPF_LIB_SUFFIX) && defined(OSPF_LIB_EXTENSION)
#pragma comment(lib, OSPF_LIB_PREFIX OSPF_LIB_NAME OSPF_LIB_SUFFIX OSPF_LIB_EXTENSION)
#endif
#endif

#undef OSPF_LIB_PREFIX
#undef OSPF_LIB_SUFFIX
#undef OSPF_LIB_EXTENSION
#endif

#undef OSPF_LIB_NAME

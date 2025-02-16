#pragma once

#include <boost/config.hpp>

#ifdef BOOST_MSVC

#if !(defined(OSPF_DYN_LINK) & (defined(USE_OSPF_BASE_LIB) || defined(USE_OSPF_LIB)))
#pragma comment(lib, "jvm.lib")
#pragma comment(lib, "jawt.lib")
#endif

#endif

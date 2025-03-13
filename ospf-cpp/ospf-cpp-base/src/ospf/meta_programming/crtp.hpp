#pragma once

#include <boost/config.hpp>

#ifndef OSPF_CRTP_PERMISSION
#ifdef BOOST_MSVC
#define OSPF_CRTP_PERMISSION protected
#else
#define OSPF_CRTP_PERMISSION public
#endif
#endif

#ifndef OSPF_CRTP_IMPL
#define OSPF_CRTP_IMPL private:\
    inline constexpr Self& self(void) & noexcept\
    {\
        return static_cast<Self&>(*this);\
    }\
    inline constexpr const Self& self(void) const & noexcept\
    {\
        return static_cast<const Self&>(*this);\
    }\
    inline Self&& self(void) && noexcept\
    {\
        return static_cast<Self&&>(*this);\
    }
#endif

#ifndef OSPF_CRTP_FUNCTION
#ifdef BOOST_MSVC
#define OSPF_CRTP_FUNCTION(F) F
#else
#define OSPF_CRTP_FUNCTION(F) _##F
#endif
#endif

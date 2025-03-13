#pragma once

#include <ospf/error/code.hpp>
#include <ospf/error/error.hpp>

namespace ospf
{
    using OSPFError = Error<OSPFErrCode, char>;

    template<typename T>
    using ExOSPFError = ExError<OSPFErrCode, T, char>;
};

#include <ospf/error/error.hpp>

namespace ospf::error
{
    template class Error<OSPFErrCode, char>;
    template class Error<OSPFErrCode, wchar>;
    //template class Error<OSPFErrCode, u8char>;
    //template class Error<OSPFErrCode, u16char>;
    //template class Error<OSPFErrCode, u32char>;
};

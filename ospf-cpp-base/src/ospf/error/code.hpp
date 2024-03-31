#pragma once

#include <ospf/basic_definition.hpp>
#include <ospf/concepts/enum.hpp>
#include <ospf/concepts/with_default.hpp>
#include <ospf/literal_constant.hpp>

namespace ospf
{
    inline namespace error
    {
        // todo: complete it and replace application error
        enum class OSPFErrCode : u8
        {
            None = 0x00_u8,

            NotAFile = 0x10_u8,
            NotADirectory = 0x11_u8,
            FileNotFound = 0x12_u8,
            DirectoryUnusable = 0x13_u8,
            FileExtensionNotMatched = 0x14_u8,
            DataNotFound = 0x15_u8,
            DataEmpty = 0x16_u8,
            EnumVisitorEmpty = 0x17_u8,
            UniquePtrLocked = 0x18_u8,
            UniqueBorrowLocked = 0x19_u8,
            SerializationFail = 0x1a_u8,
            DeserializationFail = 0x1b_u8,

            LackOfPipelines = 0x20_u8,
            SolverNotFound = 0x21_u8,
            OREngineEnvironmentLost = 0x22_u8,
            OREngineConnectionOvertime = 0x23_u8,
            OREngineModelingException = 0x24_u8,
            OREngineSolvingException = 0x25_u8,
            OREngineTerminated = 0x26_u8,
            ORModelNoSolution = 0x27_u8,
            ORModelUnbounded = 0x28_u8,
            ORSolutionInvalid = 0x29_u8,

            ApplicationFail = 0x30_u8,
            ApplicationError = 0x31_u8,
            ApplicationException = 0x32_u8,
            ApplicationStop = 0x33_u8,

            Other = ((std::numeric_limits<u8>::max)()) - 1,
            Unknown = ((std::numeric_limits<u8>::max)())
        };

        inline constexpr const bool operator==(const OSPFErrCode lhs, const OSPFErrCode rhs) noexcept
        {
            return static_cast<u8>(lhs) == static_cast<u8>(rhs);
        }

        inline constexpr const bool operator!=(const OSPFErrCode lhs, const OSPFErrCode rhs) noexcept
        {
            return static_cast<u8>(lhs) != static_cast<u8>(rhs);
        }
    };
};

namespace ospf
{
    template<>
    struct DefaultValue<OSPFErrCode>
    {
        inline static constexpr const OSPFErrCode value(void) noexcept
        {
            return OSPFErrCode::None;
        }
    };
};

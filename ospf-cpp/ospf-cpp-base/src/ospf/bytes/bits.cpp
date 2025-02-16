#include <ospf/bytes/bits.hpp>

namespace ospf::bytes::bits
{
	const u8 reverse_bits_u8(u8 value) noexcept
	{
		value = ((value & 0x0f) << 4) | ((value & 0xf0) >> 4);
		value = ((value & 0x33) << 2) | ((value & 0xcc) >> 2);
		value = ((value & 0x55) << 1) | ((value & 0xaa) >> 1);
		return value;
	}

	const u16 reverse_bits_u16(u16 value) noexcept
	{
		value = ((value & 0xAAAA) >> 1) | ((value & 0x5555) << 1);
		value = ((value & 0xCCCC) >> 2) | ((value & 0x3333) << 2);
		value = ((value & 0xF0F0) >> 4) | ((value & 0x0F0F) << 4);
		value = ((value & 0xFF00) >> 8) | ((value & 0x00FF) << 8);
		return value;
	}

	const u32 reverse_bits_u32(u32 value) noexcept
	{
		value = ((value & 0xAAAAAAAA) >> 1) | ((value & 0x55555555) << 1);
		value = ((value & 0xCCCCCCCC) >> 2) | ((value & 0x33333333) << 2);
		value = ((value & 0xF0F0F0F0) >> 4) | ((value & 0x0F0F0F0F) << 4);
		value = ((value & 0xFF00FF00) >> 8) | ((value & 0x00FF00FF) << 8);
		value = ((value & 0xFFFF0000) >> 16) | ((value & 0x0000FFFF) << 16);
		return value;
	}

	const u64 reverse_bits_u64(u64 value) noexcept
	{
		value = ((value & 0xAAAAAAAAAAAAAAAA) >> 1) | ((value & 0x5555555555555555) << 1);
		value = ((value & 0xCCCCCCCCCCCCCCCC) >> 2) | ((value & 0x3333333333333333) << 2);
		value = ((value & 0xF0F0F0F0F0F0F0F0) >> 4) | ((value & 0x0F0F0F0F0F0F0F0F) << 4);
		value = ((value & 0xFF00FF00FF00FF00) >> 8) | ((value & 0x00FF00FF00FF00FF) << 8);
		value = ((value & 0xFFFF0000FFFF0000) >> 16) | ((value & 0x0000FFFF0000FFFF) << 16);
		value = ((value & 0xFFFFFFFF00000000) >> 32) | ((value & 0x00000000FFFFFFFF) << 32);
		return value;
	}
};
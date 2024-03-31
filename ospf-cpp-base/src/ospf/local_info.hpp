#pragma once

#include <ospf/basic_definition.hpp>
#include <magic_enum.hpp>

#ifndef OSPF_LANG
#if defined(OSPF_LANG_EN)
#define OSPF_LANG EN
#define OSPF_LANG_CODE en
#elif defined(OSPF_LANG_FR)
#define OSPF_LANG FR
#define OSPF_LANG_CODE fr
#elif defined(OSPF_LANG_RU)
#define OSPF_LANG RU
#define OSPF_LANG_CODE ru
#elif defined(OSPF_LANG_ZH)
#define OSPF_LANG ZH
#define OSPF_LANG_CODE zh
#elif defined(LANG_EN)
#define OSPF_LANG_EN
#define OSPF_LANG EN
#define OSPF_LANG_CODE en
#elif defined(LANG_FR)
#define OSPF_LANG_FR
#define OSPF_LANG FR
#define OSPF_LANG_CODE fr
#elif defined(LANG_RU)
#define OSPF_LANG_RU
#define OSPF_LANG RU
#define OSPF_LANG_CODE ru
#elif defined(LANG_ZH)
#define OSPF_LANG_ZH
#define OSPF_LANG ZH
#define OSPF_LANG_CODE zh
#else
#define OSPF_LANG_EN
#define OSPF_LANG EN
#define OSPF_LANG_CODE en
#endif
#endif

#ifndef OSPF_COUNTRY_REGION
#if defined(OSPF_COUNTRY_REGION_CHN)
#define OSPF_COUNTRY_REGION CHN
#elif defined(OSPF_COUNTRY_REGION_FRA)
#define OSPF_COUNTRY_REGION FRA
#elif defined(OSPF_COUNTRY_REGION_GBR)
#define OSPF_COUNTRY_REGION GBR
#elif defined(OSPF_COUNTRY_REGION_RUS)
#define OSPF_COUNTRY_REGION RUS
#elif defined(OSPF_COUNTRY_REGION_USA)
#define OSPF_COUNTRY_REGION USA
#elif defined(COUNTRY_CHN)
#define OSPF_COUNTRY_REGION_CHN
#define OSPF_COUNTRY_REGION CHN
#define OSPF_COUNTRY_REGION_CODE CN
#elif defined(COUNTRY_FRA)
#define OSPF_COUNTRY_REGION_FRA
#define OSPF_COUNTRY_REGION FRA
#define OSPF_COUNTRY_REGION_CODE FR
#elif defined(COUNTRY_GBR)
#define OSPF_COUNTRY_REGION_GBR
#define OSPF_COUNTRY_REGION GBR
#define OSPF_COUNTRY_REGION_CODE GB
#elif defined(COUNTRY_RUS)
#define OSPF_COUNTRY_REGION_RUS
#define OSPF_COUNTRY_REGION RUS
#define OSPF_COUNTRY_REGION_CODE RU
#elif defined(COUNTRY_USA)
#define OSPF_COUNTRY_REGION_USA
#define OSPF_COUNTRY_REGION USA
#define OSPF_COUNTRY_REGION_CODE US
#else
#define OSPF_COUNTRY_REGION_CHN
#define OSPF_COUNTRY_REGION CHN
#define OSPF_COUNTRY_REGION_CODE CN
#endif
#endif

namespace ospf
{
    enum class Language : u8
    {
        English,
        French,
        Russian,
        Chinese
    };

    enum class CountryRegion : u8
    {
        CHN,
        FRA,
        GBR,
        RUS,
        USA,
    };

    enum class Charset : u8
    {
        ASCII,
        GB2312,
        GBK,
        UTF8,
        UTF16
    };

#ifdef OSPF_LANG_EN
    static constexpr Language default_language = Language::English;
#elif defined(OSPF_LANG_FR)
    static constexpr Language default_language = Language::French;
#elif defined(OSPF_LANG_RU)
    static constexpr Language default_language = Lnaguage::Russian;
#elif defined(OSPF_LANG_ZH)
    static constexpr Language default_language = Language::Chinese;
#endif

#ifdef OSPF_COUNTRY_REGION_CHN
    static constexpr CountryRegion default_country_region = CountryRegion::CHN;
#elif defined(OSPF_COUNTRY_REGION_FRA)
    static constexpr CountryRegion default_country_region = CountryRegion::FRA;
#elif defined(OSPF_COUNTRY_REGION_GBR)
    static constexpr CountryRegion default_country_region = CountryRegion::GBR;
#elif defined(OSPF_COUNTRY_REGION_RUS)
    static constexpr CountryRegion default_country_region = CountryRegion::RUS;
#elif defined(OSPF_COUNTRY_REGION_USA)
    static constexpr CountryRegion default_country_region = CountryRegion::USA;
#endif

    static constexpr Charset default_charset = Charset::UTF8;
};

namespace magic_enum::customize
{
    template<>
    constexpr customize_t enum_name<ospf::Charset>(const ospf::Charset value) noexcept
    {
        switch (value) {
        case ospf::Charset::ASCII:
            return "ASCII";
        case ospf::Charset::GB2312:
            return "GB2312";
        case ospf::Charset::GBK:
            return "GBK";
        case ospf::Charset::UTF8:
            return "UTF-8";
        case ospf::Charset::UTF16:
            return "UTF-16";
        default:
            return default_tag;
        }
    }
};

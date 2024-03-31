#pragma once

#include <ospf/ospf_base_api.hpp>
#include <jni.h>
#include <string>

namespace ospf
{
    inline namespace jni
    {
        OSPF_BASE_API std::string jstr_to_string(JNIEnv* env, jstring jstr);
        OSPF_BASE_API jstring string_to_jstr(JNIEnv* env, const std::string_view str);
    };
};

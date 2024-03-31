#include <ospf/jni/jstring.hpp>
#include <ospf/system_info.hpp>
#include <iterator>

namespace ospf::jni
{
    namespace jni_detail
    {
        inline static constexpr const std::string_view get_jni_encoding_code(const OperationSystem os)
        {
            return local_os == OperationSystem::Windows ? "GB2312" : "UTF-8";
        }
    };

    std::string jstr_to_string(JNIEnv* env, jstring jstr)
    {
        auto class_id = env->FindClass("Ljava/lang/String;");
        auto method_id = env->GetMethodID(class_id, "getBytes", "(Ljava/lang/String;)[B");
        auto encode = env->NewStringUTF(jni_detail::get_jni_encoding_code(local_os).data());
        auto byte_array = (jbyteArray)env->CallObjectMethod(jstr, method_id, encode);
        auto len = static_cast<usize>(env->GetArrayLength(byte_array));
        auto* bytes = env->GetByteArrayElements(byte_array, JNI_FALSE);

        std::string ret;
        if (len > 0)
        {
            ret.reserve(len + 1);
            std::copy(bytes, bytes + len, std::back_inserter(ret));
        }
        env->ReleaseByteArrayElements(byte_array, bytes, 0);
        return ret;
    }

    jstring string_to_jstr(JNIEnv* env, const std::string_view str)
    {
        auto class_id = env->FindClass("Ljava/lang/String;");
        auto construct = env->GetMethodID(class_id, "<init>", "([BLjava/lang/String;)V");
        auto bytes = env->NewByteArray(static_cast<jsize>(str.size()));
        env->SetByteArrayRegion(bytes, 0, static_cast<jsize>(str.size()), reinterpret_cast<const jbyte*>(str.data()));
        return (jstring)env->NewObject(class_id, construct, bytes, env->NewStringUTF(jni_detail::get_jni_encoding_code(local_os).data()));
    }
};

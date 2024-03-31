import sys
import platform
import os
import shutil
import fnmatch

def ignore_pattern(include_patterns: list, ignore_patterns: list = []):
    def _ignore_patterns(path, names):
        keep = set(name for pattern in include_patterns
                            for name in fnmatch.filter(names, pattern)) \
                - set(name for pattern in ignore_patterns
                            for name in fnmatch.filter(names, pattern))
        ignore = set(name for name in names
                        if name not in keep and not os.path.isdir(os.path.join(path, name)))
        return ignore
    return _ignore_patterns


def copy_plugin_src(path: str, out_path: str):
    os.makedirs(out_path)
    for child_path in os.listdir(path):
        if not os.path.isfile(os.path.abspath("%s/%s" % (path, child_path))):
            continue
        (filename, extension) = os.path.splitext(child_path)
        if extension != ".hpp" and extension != ".h":
            continue
        if filename[-4:] == "impl":
            continue
        shutil.copyfile(os.path.abspath("%s/%s" % (path, child_path)), os.path.abspath("%s/%s" % (out_path, child_path)))


def copy_lib(path: str, out_path: str, target_extensions: list):
    for child_path in os.listdir(path):
        if not os.path.isfile(os.path.abspath("%s/%s" % (path, child_path))):
            continue
        (filename, extension) = os.path.splitext(child_path)
        if extension not in target_extensions:
            continue
        if "ospf" not in filename:
            continue
        if "example" in filename:
            continue
        if "csharp" in filename:
            continue
        if "test" in filename:
            continue
        shutil.copyfile(os.path.abspath("%s/%s" % (path, child_path)), os.path.abspath("%s/%s" % (out_path, child_path)))


def cpp_copy_src(path: str, out_path: str):
    os.mkdir(out_path)

    shutil.copytree(
        os.path.abspath("%s/ospf" % path), 
        os.path.abspath("%s/ospf" % out_path),
        ignore=ignore_pattern(['*.h', '*.hpp'])
    )


def cpp_copy_lib(path: str, out_path: str):
    if not os.path.exists(out_path):
        os.makedirs(out_path)
    
    copy_lib(path, out_path, [".lib", ".a", ".so"])


def cpp_copy_bin(path: str, out_path: str):
    if not os.path.exists(out_path):
        os.makedirs(out_path)

    copy_lib(path, out_path, [".dll", ""])


def main():
    curr_path = sys.path[0]
    cpp_src_path = os.path.abspath("%s/src" % curr_path)

    if "Windows" in platform.platform():
        # os.system("sh compile.bat")

        cpp_release_out_path = os.path.abspath("%s/x64/Release" % curr_path)
        cpp_debug_out_path = os.path.abspath("%s/x64/Debug" % curr_path)
        release_path = os.path.abspath("%s/release" % curr_path)
        if os.path.exists(release_path):
            shutil.rmtree(release_path)
        os.makedirs(release_path)
        cpp_release_path = os.path.abspath("%s/cpp/win_x64_msvc142" % release_path)
        os.makedirs(cpp_release_path)

        cpp_copy_src(cpp_src_path, os.path.abspath("%s/include" % cpp_release_path))
        cpp_copy_lib(cpp_release_out_path, os.path.abspath("%s/lib" % cpp_release_path))
        cpp_copy_bin(cpp_release_out_path, os.path.abspath("%s/bin" % cpp_release_path))
        cpp_copy_lib(cpp_debug_out_path, os.path.abspath("%s/lib" % cpp_release_path))
        cpp_copy_bin(cpp_debug_out_path, os.path.abspath("%s/bin" % cpp_release_path))
    else:
        cpp_release_out_path = os.path.abspath("%s/x64/Release" % curr_path)
        cpp_debug_out_path = os.path.abspath("%s/x64/Debug" % curr_path)

        if not os.path.exists(cpp_release_out_path):
            os.makedirs("%s/x64/Release/build" % curr_path)
        else:
            shutil.rmtree("%s/x64/Release/bin" % curr_path)
            shutil.rmtree("%s/x64/Release/lib" % curr_path)
        os.makedirs("%s/x64/Release/bin" % curr_path)
        os.makedirs("%s/x64/Release/lib" % curr_path)

        if not os.path.exists(cpp_debug_out_path):
            os.makedirs("%s/x64/Debug/build" % curr_path)
        else:
            shutil.rmtree("%s/x64/Debug/bin" % curr_path)
            shutil.rmtree("%s/x64/Debug/lib" % curr_path)
        os.makedirs("%s/x64/Debug/bin" % curr_path)
        os.makedirs("%s/x64/Debug/lib" % curr_path)

        os.system("sh compile.sh")

        release_path = os.path.abspath("%s/release" % curr_path)
        if os.path.exists(release_path):
            shutil.rmtree(release_path)
        os.makedirs(release_path)
        cpp_release_path = os.path.abspath("%s/cpp/unix_x64_gcc10" % release_path)
        # cpp_release_path = os.path.abspath("%s/cpp/unix_x64_clang11" % release_path)
        os.makedirs(cpp_release_path)

        cpp_copy_src(cpp_src_path, os.path.abspath("%s/include" % cpp_release_path))
        cpp_copy_lib("%s/lib" % cpp_release_out_path, os.path.abspath("%s/lib" % cpp_release_path))
        cpp_copy_bin("%s/bin" % cpp_release_out_path, os.path.abspath("%s/bin" % cpp_release_path))
        cpp_copy_lib("%s/lib" % cpp_debug_out_path, os.path.abspath("%s/lib" % cpp_release_path))
        cpp_copy_bin("%s/bin" % cpp_debug_out_path, os.path.abspath("%s/bin" % cpp_release_path))


if __name__ == "__main__":
    main()

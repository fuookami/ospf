#pragma once

#include <ospf/config.hpp>
#include <ospf/log/interface.hpp>
#include <ospf/functional/result.hpp>
#include <fstream>
#include <filesystem>

namespace ospf
{
    inline namespace log
    {
        template<
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class DynFileLogger
            : public log_detail::DynLoggerImpl<mt, CharT, DynFileLogger<mt, CharT>>
        {
            using Impl = log_detail::DynLoggerImpl<mt, CharT, DynFileLogger<mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            DynFileLogger(const LogLevel lowest_level, std::basic_ofstream<CharT> fout, const typename RecordType::WriterGenerator& writer_generator)
                : Impl(lowest_level), _fout(std::move(fout)), _writer(writer_generator(_fout)) {}

            template<typename = void>
                requires (mt == on)
            DynFileLogger(const LogLevel lowest_level, std::basic_ofstream<CharT> fout, const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(lowest_level, with_buffer), _fout(std::move(fout)), _writer(writer_generator(_fout)) {}

        public:
            DynFileLogger(const DynFileLogger& ano) = delete;
            DynFileLogger(DynFileLogger&& ano) noexcept = default;
            DynFileLogger& operator=(const DynFileLogger& rhs) = delete;
            DynFileLogger& operator=(DynFileLogger&& rhs) = delete;
            ~DynFileLogger(void) = default;

        public:
            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(_fout);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                _fout << std::move(message);
            }

        private:
            std::basic_ofstream<CharT> _fout;
            RecordType::Writer _writer;
        };

        template<
            LogLevel lowest_level = default_log_level,
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        class FileLogger
            : public log_detail::LoggerImpl<lowest_level, mt, CharT, FileLogger<lowest_level, mt, CharT>>
        {
            using Impl = log_detail::LoggerImpl<lowest_level, mt, CharT, FileLogger<lowest_level, mt, CharT>>;

        public:
            using typename Impl::RecordType;
            using typename Impl::StringType;
            using typename Impl::StringViewType;

        public:
            template<typename = void>
                requires (mt == off)
            FileLogger(std::basic_ofstream<CharT> fout, const typename RecordType::WriterGenerator& writer_generator)
                : _fout(std::move(fout)), _writer(writer_generator(_fout)) {}

            template<typename = void>
                requires (mt == on)
            FileLogger(std::basic_ofstream<CharT> fout, const typename RecordType::WriterGenerator& writer_generator, const bool with_buffer = true)
                : Impl(with_buffer), _fout(std::move(fout)), _writer(writer_generator(_fout)) {}

        public:
            FileLogger(const FileLogger& ano) = delete;
            FileLogger(FileLogger&& ano) noexcept = default;
            FileLogger& operator=(const FileLogger& rhs) = delete;
            FileLogger& operator=(FileLogger&& rhs) = delete;
            ~FileLogger(void) = default;

        public:
            inline void set_writer(const typename RecordType::WriterGenerator& writer_generator) noexcept
            {
                if constexpr (mt == on)
                {
                    this->flush();
                }
                _writer = writer_generator(_fout);
            }

        OSPF_CRTP_PERMISSION:
            RecordType make_record(const LogLevel level, StringType message) const noexcept
            {
                return RecordType{ level, std::move(message), _writer };
            }

            void write_message(StringType message) noexcept
            {
                _fout << std::move(message);
            }

        private:
            std::basic_ofstream<CharT> _fout;
            RecordType::Writer _writer;
        };

        template<
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        inline Result<LoggerUniqueWrapper<CharT>> make_dyn_file_logger(const std::filesystem::path& path, const LogLevel lowest_level, const typename LogRecord<CharT>::WriterGenerator& writer_generator) noexcept
        {
            if (std::filesystem::is_directory(path))
            {
                return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path) };
            }

            const auto parent_path = path.parent_path();
            if (!std::filesystem::exists(parent_path))
            {
                if (!std::filesystem::create_directories(parent_path))
                {
                    return OSPFError{ OSPFErrCode::DirectoryUnusable, std::format("directory \"{}\" unusable", parent_path) };
                }
            }

            std::basic_ofstream<CharT> fout{ path };
            auto new_logger = new DynFileLogger<mt, CharT>{ lowest_level, std::move(fout), writer_generator };
            return LoggerUniqueWrapper<CharT>{ Unique<DynLoggerInterface<CharT>>{ new_logger } };
        }

        template<
            LogLevel lowest_level = default_log_level,
            LoggerMultiThread mt = OSPF_BASE_MULTI_THREAD,
            CharType CharT = char
        >
        inline Result<LoggerUniqueWrapper<CharT>> make_file_logger(const std::filesystem::path& path, const typename LogRecord<CharT>::WriterGenerator& writer_generator) noexcept
        {
            if (std::filesystem::is_directory(path))
            {
                return OSPFError{ OSPFErrCode::NotAFile, std::format("\"{}\" is not a file", path) };
            }

            const auto parent_path = path.parent_path();
            if (!std::filesystem::exists(parent_path))
            {
                if (!std::filesystem::create_directories(parent_path))
                {
                    return OSPFError{ OSPFErrCode::DirectoryUnusable, std::format("directory \"{}\" unusable", parent_path) };
                }
            }

            std::basic_ofstream<CharT> fout{ path };
            auto new_logger = new FileLogger<lowest_level, mt, CharT>{ std::move(fout), writer_generator };
            return LoggerUniqueWrapper<CharT>{ Unique<DynLoggerInterface<CharT>>{ new_logger } };
        }

        extern template class DynFileLogger<on, char>;
        extern template class DynFileLogger<off, char>;
        extern template class DynFileLogger<on, wchar>;
        extern template class DynFileLogger<off, wchar>;

        extern template class FileLogger<LogLevel::Trace, on, char>;
        extern template class FileLogger<LogLevel::Trace, off, char>;
        extern template class FileLogger<LogLevel::Trace, on, wchar>;
        extern template class FileLogger<LogLevel::Trace, off, wchar>;

        extern template class FileLogger<LogLevel::Debug, on, char>;
        extern template class FileLogger<LogLevel::Debug, off, char>;
        extern template class FileLogger<LogLevel::Debug, on, wchar>;
        extern template class FileLogger<LogLevel::Debug, off, wchar>;

        extern template class FileLogger<LogLevel::Info, on, char>;
        extern template class FileLogger<LogLevel::Info, off, char>;
        extern template class FileLogger<LogLevel::Info, on, wchar>;
        extern template class FileLogger<LogLevel::Info, off, wchar>;

        extern template class FileLogger<LogLevel::Warn, on, char>;
        extern template class FileLogger<LogLevel::Warn, off, char>;
        extern template class FileLogger<LogLevel::Warn, on, wchar>;
        extern template class FileLogger<LogLevel::Warn, off, wchar>;

        extern template class FileLogger<LogLevel::Error, on, char>;
        extern template class FileLogger<LogLevel::Error, off, char>;
        extern template class FileLogger<LogLevel::Error, on, wchar>;
        extern template class FileLogger<LogLevel::Error, off, wchar>;

        extern template class FileLogger<LogLevel::Fatal, on, char>;
        extern template class FileLogger<LogLevel::Fatal, off, char>;
        extern template class FileLogger<LogLevel::Fatal, on, wchar>;
        extern template class FileLogger<LogLevel::Fatal, off, wchar>;
    };
};

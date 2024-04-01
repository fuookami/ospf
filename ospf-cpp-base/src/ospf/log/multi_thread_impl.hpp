#pragma once

#include <ospf/memory/pointer.hpp>
#include <ospf/log/record.hpp>
#include <ospf/ospf_base_api.hpp>
#include <ospf/random.hpp>
#include <atomic>
#include <condition_variable>
#include <mutex>
#include <thread>
#include <vector>

namespace ospf
{
    inline namespace log
    {
        namespace log_detail
        {
            template<CharType CharT>
            class MultiThreadImpl
            {
            public:
                MultiThreadImpl(const bool with_buffer)
                    : _with_buffer(with_buffer), _finished(false)
                {
                    _records.reserve(_with_buffer ? 16_uz : 4_uz);
                    _thread = make_unique<std::thread>([this]()
                        {
                            worker(this->_finished, this->_mutex, this->_record_condition, this->_records);
                        });
                }

            public:
                MultiThreadImpl(const MultiThreadImpl& ano) = delete;
                MultiThreadImpl(MultiThreadImpl&& ano) noexcept = default;
                MultiThreadImpl& operator=(const MultiThreadImpl& rhs) = delete;
                MultiThreadImpl& operator=(MultiThreadImpl&& rhs) = delete;

            public:
                ~MultiThreadImpl(void)
                {
                    if (!_finished)
                    {
                        join();
                    }
                }

            public:
                inline const std::vector<LogRecord<CharT>>& waiting_records(void) const noexcept
                {
                    return _records;
                }

                inline void add(LogRecord<CharT> record) noexcept
                {
                    static auto gen = random_generator<u64>();
                    std::unique_lock<std::mutex> lck{ _mutex };
                    _records.push_back(std::move(record));
                    if (!_with_buffer || gen() % 4_u64 == 0_u64)
                    {
                        _record_condition.notify_one();
                    }
                }

                inline void join(void) noexcept
                {
                    _finished = true;
                    _record_condition.notify_one();
                    _thread->join();
                }

                inline void flush(void) noexcept
                {
                    _record_condition.notify_one();
                    std::this_thread::sleep_for(std::chrono::milliseconds{ 1 });
                    std::unique_lock<std::mutex> lck{ _mutex };
                }

            private:
                inline static void worker(std::atomic<bool>& finished, std::mutex& mutex, std::condition_variable& record_cond, std::vector<LogRecord<CharT>>& records) noexcept
                {
                    while (!finished)
                    {
                        std::unique_lock<std::mutex> lck{ mutex };
                        record_cond.wait(lck, [&finished, &records]()
                            {
                                return finished || !records.empty();
                            });

                        std::vector<LogRecord<CharT>> this_records;
                        std::swap(this_records, records);
                        lck.release()->unlock();

                        for (const auto& record : this_records)
                        {
                            record.write();
                        }
                    }

                    for (const auto& record : records)
                    {
                        record.write();
                    }
                    records.clear();
                }

            private:
                bool _with_buffer;
                std::atomic<bool> _finished;
                std::mutex _mutex;
                std::condition_variable _record_condition;
                Unique<std::thread> _thread;
                std::vector<LogRecord<CharT>> _records;
            };

            extern template class MultiThreadImpl<char>;
            extern template class MultiThreadImpl<wchar>;
        };
    };
};

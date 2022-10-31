#include "js_bindings.h"

#include <emscripten.h>

#include <cstdio>
#include <cstring>
#include <ostream>
#include <streambuf>

EM_JS(void, jPrint, (const char* content),
      { jsPrint(UTF8ToString(content)); });

// Borrowed code from http://videocortex.io/2017/custom-stream-buffers/
template <typename Callback>
class CallbackOstreamBuffer : public std::streambuf {
   public:
    using callback_t = Callback;
    CallbackOstreamBuffer(Callback cb, void* user_data = nullptr)
        : callback_(cb), user_data_(user_data) {}

   protected:
    std::streamsize xsputn(const char_type* s,
                           std::streamsize n) override {
        return callback_(s, n,
                         user_data_);  // returns the number of characters
                                       // successfully written.
    };

    int_type overflow(int_type ch) override {
        return callback_(&ch, 1,
                         user_data_);  // returns the number of characters
                                       // successfully written.
    }

   private:
    Callback callback_;
    void* user_data_;
};

static CallbackOstreamBuffer jout_buf([](const void* buf,
                                         std::streamsize sz,
                                         void* user_data) {
    (void)sz;
    (void)user_data;
    auto sbuf = static_cast<const char*>(buf);
    jPrint(sbuf);
    return strlen(sbuf);
});

static CallbackOstreamBuffer jerr_buf([](const void* buf,
                                         std::streamsize sz,
                                         void* user_data) {
    (void)sz;
    (void)user_data;
    auto sbuf = static_cast<const char*>(buf);
    jPrint(sbuf);
    return strlen(sbuf);
});

std::shared_ptr<std::ostream> jout =
    std::make_shared<std::ostream>(&jout_buf);
std::shared_ptr<std::ostream> jerr =
    std::make_shared<std::ostream>(&jerr_buf);

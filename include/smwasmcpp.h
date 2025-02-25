
typedef char* (*SmCallFunction)(const char*);

extern "C" {
    int sm_cn();
    void sm_load(const char* szWasm, int nSpace);
    void sm_register(const char* szDefine, SmCallFunction fnCallback);
    char* sm_call(const char* szUsage);
}

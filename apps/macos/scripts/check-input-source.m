//! 独立诊断工具：检查系统切换浮窗使用的输入源查找接口，不选择输入源、不发送按键。
// 私有接口仅用于复现系统崩溃链路，不链接进青简产品；接口不存在时返回 2。
#import <AppKit/AppKit.h>
#import <Carbon/Carbon.h>
#include <dlfcn.h>

int main(int argc, const char *argv[]) {
    @autoreleasepool {
        [NSApplication sharedApplication];
        TISInputSourceRef (*copyByID)(CFStringRef) =
            dlsym(RTLD_DEFAULT, "TISCopyInputSourceRefForInputSourceID");
        if (!copyByID) {
            fprintf(stderr, "当前系统没有可用的浮窗诊断接口。\n");
            return 2;
        }
        NSString *identifier = argc > 1 ? [NSString stringWithUTF8String:argv[1]]
                                       : @"app.qingjian.inputmethod";
        if (!identifier.length) return 2;
        // 必须是本进程首次输入源查询，避免其他查询预先刷新缓存掩盖故障。
        TISInputSourceRef source = copyByID((__bridge CFStringRef)identifier);
        if (!source) {
            fprintf(stderr, "%s：查找返回 NULL，系统切换浮窗可能因此崩溃。\n", identifier.UTF8String);
            return 1;
        }
        CFRelease(source);
        printf("%s：输入源查找成功。\n", identifier.UTF8String);
        return 0;
    }
}

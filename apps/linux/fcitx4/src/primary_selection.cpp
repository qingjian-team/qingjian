//! X11 PRIMARY 选区备用读取；只在用户按下翻译快捷键时调用。
#include "primary_selection.h"
#include "../../surrounding.h"
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <X11/extensions/XRes.h>
#include <poll.h>
#include <chrono>
#include <string>
#include <utility>

namespace qingjian {
namespace {
bool sameClient(Display *display, Window owner, Window focus) {
    int event = 0, error = 0, count = 0;
    XResClient *clients = nullptr;
    if (!XResQueryExtension(display, &event, &error)
        || !XResQueryClients(display, &count, &clients)) return false;
    XID ownerBase = 0, focusBase = 0;
    for (int i = 0; i < count; ++i) {
        const auto base = clients[i].resource_base;
        const auto mask = clients[i].resource_mask;
        if ((owner & ~mask) == base) ownerBase = base;
        if ((focus & ~mask) == base) focusBase = base;
    }
    if (clients) XFree(clients);
    return ownerBase && ownerBase == focusBase;
}
}

std::string primarySelection() {
    Display *display = XOpenDisplay(nullptr);
    if (!display) return {};
    const Window owner = XGetSelectionOwner(display, XA_PRIMARY);
    Window focus = None;
    int revert = 0;
    XGetInputFocus(display, &focus, &revert);
    if (owner == None || focus == None || focus == PointerRoot || !sameClient(display, owner, focus)) {
        XCloseDisplay(display);
        return {};
    }

    const Window requestor = XCreateSimpleWindow(display, DefaultRootWindow(display), 0, 0, 1, 1, 0, 0, 0);
    const Atom utf8 = XInternAtom(display, "UTF8_STRING", False);
    const Atom property = XInternAtom(display, "QINGJIAN_PRIMARY_SELECTION", False);
    XConvertSelection(display, XA_PRIMARY, utf8, property, requestor, CurrentTime);
    XFlush(display);

    std::string result;
    const auto deadline = std::chrono::steady_clock::now() + std::chrono::milliseconds(100);
    XEvent event{};
    while (std::chrono::steady_clock::now() < deadline) {
        if (XCheckTypedWindowEvent(display, requestor, SelectionNotify, &event)) {
            Window currentFocus = None;
            XGetInputFocus(display, &currentFocus, &revert);
            if (event.xselection.property == property && XGetSelectionOwner(display, XA_PRIMARY) == owner
                && currentFocus == focus) {
                Atom type = None;
                int format = 0;
                unsigned long items = 0, remaining = 0;
                unsigned char *data = nullptr;
                if (XGetWindowProperty(display, requestor, property, 0, 2048, False, utf8,
                        &type, &format, &items, &remaining, &data) == Success && type == utf8
                    && format == 8 && remaining == 0 && items <= 2000 && data) {
                    std::string text(reinterpret_cast<char *>(data), items);
                    if (text.find('\0') == std::string::npos && !surrounding(text, 0, 0).is_null())
                        result = std::move(text);
                }
                if (data) XFree(data);
            }
            break;
        }
        const auto left = std::chrono::duration_cast<std::chrono::milliseconds>(deadline - std::chrono::steady_clock::now());
        if (left.count() <= 0) break;
        pollfd descriptor{ConnectionNumber(display), POLLIN, 0};
        if (poll(&descriptor, 1, static_cast<int>(left.count())) <= 0) break;
    }
    XDestroyWindow(display, requestor);
    XCloseDisplay(display);
    return result;
}
}

-- 用法：osascript select-input.applescript 青简：在菜单栏的输入法菜单里点名字含这几个字的那一项。没有这一项时报错退出。
-- 菜单路径与 TextInputMenuAgent 的结构见 docs/notes/testing.md；sshd 要有辅助功能权限。
on run argv
	set wanted to item 1 of argv
	tell application "System Events" to tell process "TextInputMenuAgent"
		set inputMenu to menu bar item 1 of menu bar 2
		click inputMenu
		delay 0.5
		repeat with candidate in menu items of menu 1 of inputMenu
			if (name of candidate as text) contains wanted then
				click candidate
				delay 0.5
				return "已切到 " & (name of candidate as text)
			end if
		end repeat
		key code 53
	end tell
	error "输入法菜单里没有 " & wanted
end run

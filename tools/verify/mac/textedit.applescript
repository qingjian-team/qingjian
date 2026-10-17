-- 用法：osascript textedit.applescript "nihao SPACE"：新建 TextEdit 文档、按记号发键、返回文档文本、不存盘关掉。
-- 记号与 tools/verify/cases.tsv 相同：SPACE / RET / ESC / BS / WAIT，其余按字符敲。前提：当前输入法已是青简。
on run argv
	set tokens to words of (item 1 of argv)
	tell application "TextEdit"
		activate
		make new document
	end tell
	delay 1
	repeat with token in tokens
		set s to token as text
		if s is "WAIT" then
			delay 1
		else
			tell application "System Events"
				if s is "SPACE" then
					key code 49
				else if s is "RET" then
					key code 36
				else if s is "ESC" then
					key code 53
				else if s is "BS" then
					key code 51
				else
					repeat with c in characters of s
						keystroke c
						delay 0.08
					end repeat
					-- 停一下再按下一个键：本地模型生成的整句是异步回来的，真人打字也会停顿（与 FreeBSD 的 e2e-app.sh 一致）
					delay 0.8
				end if
			end tell
			delay 0.3
		end if
	end repeat
	delay 0.5
	tell application "TextEdit"
		set result_text to text of front document
		close front document saving no
	end tell
	return result_text
end run

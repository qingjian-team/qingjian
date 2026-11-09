//! IBus 组件描述（`component.xml`）。
//!
//! ibus-daemon 扫 `/usr/share/ibus/component/` 下的 XML 才知道有这么个输入法、怎么把它拉起来。
//! 装包时这份 XML 要落到那里；开发时可以写进 `$IBUS_COMPONENT_PATH` 指的目录。
//! 用 `qingjian-linux --ibus-xml <可执行文件路径>` 打印。

/// D-Bus 上的组件名，也是引擎进程要占的总线名。
pub const COMPONENT_NAME: &str = "org.freedesktop.IBus.Qingjian";

/// 引擎名，`ibus engine qingjian` 用的就是它。
pub const ENGINE_NAME: &str = "qingjian";

/// 生成组件 XML。`exec` 是装好之后可执行文件的绝对路径。
pub fn xml(exec: &str) -> String {
    let version = env!("CARGO_PKG_VERSION");
    format!(
        r#"<?xml version="1.0" encoding="utf-8"?>
<!-- 青简输入法的 IBus 组件描述。装包时放 /usr/share/ibus/component/qingjian.xml -->
<component>
  <name>{COMPONENT_NAME}</name>
  <description>青简输入法</description>
  <exec>{exec} --ibus</exec>
  <version>{version}</version>
  <author>Qingjian</author>
  <license>GPL-3.0-or-later</license>
  <homepage>https://qingjian.app</homepage>
  <textdomain>qingjian</textdomain>
  <engines>
    <engine>
      <name>{ENGINE_NAME}</name>
      <language>zh_CN</language>
      <license>GPL-3.0-or-later</license>
      <author>Qingjian</author>
      <layout>us</layout>
      <longname>青简</longname>
      <description>输入的不只是文字</description>
      <rank>99</rank>
      <symbol>青</symbol>
    </engine>
  </engines>
</component>
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// XML 里的名字要与代码里用的常量一致，不然 ibus-daemon 拉起进程后对不上名字、卡在等注册。
    #[test]
    fn xml_carries_the_names_used_on_the_bus() {
        let xml = xml("/usr/bin/qingjian-linux-ibus");
        assert!(xml.contains(&format!("<name>{COMPONENT_NAME}</name>")));
        assert!(xml.contains(&format!("<name>{ENGINE_NAME}</name>")));
        assert!(xml.contains("<exec>/usr/bin/qingjian-linux-ibus --ibus</exec>"));
    }

    /// 版本跟着 crate 走，别写死。
    #[test]
    fn xml_version_follows_the_crate() {
        assert!(xml("x").contains(&format!("<version>{}</version>", env!("CARGO_PKG_VERSION"))));
    }
}

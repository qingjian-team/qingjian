<p align="center"><a href="../../README.md">简体中文</a> | <a href="README.zh-Hant.md">繁體中文</a> | <a href="README.en.md">English</a> | <a href="README.ja.md">日本語</a> | <a href="README.es.md">Español</a> | <a href="README.vi.md">Tiếng Việt</a> | <b>Deutsch</b></p>

<p align="center">
  <img src="../../assets/icon/qingjian-mark.svg" alt="Bambusstreifen-Symbol von Qingjian" height="108">
</p>

<h1 align="center">青简 Qingjian</h1>

<p align="center"><strong>Ganz normal tippen – und nebenbei ein Wort mehr lernen.</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux: Fcitx4 / 5, manueller Start"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+: Vorschau"></a>
</p>

Qingjian ist eine Eingabemethode. Du tippst wie gewohnt: Pinyin eingeben, einen Kandidaten wählen, ganze Sätze schreiben. Eine Übersetzung neben jedem Kandidaten lässt dich ganz nebenbei beim täglichen Tippen eine Sprache lernen. Die Übersetzung ist immer nur eine Zusatzinformation und tritt nie an die Stelle des Textes, den du eingibst.

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

Das Video zeigt unter macOS die Satzeingabe, das Umsortieren der Kandidaten und die Übersetzungen der Kandidaten.

## Herunterladen und loslegen

- **macOS, Windows**: [Qingjian herunterladen](https://qingjian.app/download); die Installationsschritte stehen in der [Benutzerdokumentation](https://qingjian.app/docs/getting-started/install).
- **Linux**: Fcitx5 und Fcitx4 werden aus dem Quellcode installiert und nutzen das Standard-Kandidatenfenster des Systems; der Hintergrunddienst muss derzeit von Hand gestartet werden, siehe [Installation unter Linux](https://qingjian.app/docs/platforms/linux).
- **Android**: eine Vorschau mit weniger Funktionen als die Desktop-Versionen; die arm64-APK gibt es unter [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest), zum Aktivieren siehe [Android](https://qingjian.app/docs/platforms/android), zum Bauen aus dem Quellcode [apps/android](../../apps/android/README.md).

**Offizielle Bezugsquellen**: derzeit nur die folgenden; ein „Qingjian“ von anderswo ist keine offizielle Veröffentlichung.

- [Die Download-Seite der Website](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- Der Quark-Drive-Link im Änderungsprotokoll der jeweiligen Version

Qingjian ist bisher in keinem Paketmanager veröffentlicht.

Die Versionen für macOS und Windows befinden sich noch in der Beta-Phase. Wähle nach der Installation Qingjian aus, lege unter „Einstellungen → Allgemein“ die Sprache fest, die du lernen möchtest, und fang an zu tippen. Für den Einstieg lies [Die erste Eingabe](https://qingjian.app/docs/getting-started/first-input).

## Was du beim Tippen siehst

```text
1  开发        entwickeln
2  编程        die Programmierung
3  架构        die Architektur
```

Neben den Kandidaten wird immer nur eine Lernsprache angezeigt. Derzeit stehen Englisch, Japanisch, Spanisch, Vietnamesisch und Deutsch zur Wahl; die Übersetzungen lassen sich auch ausschalten. Qingjian unterstützt außerdem Satzeingabe, abgekürztes Pinyin, Tippfehlerkorrektur, Shuangpin, Wubi und mehr; ein lokales Satzmodell überarbeitet die Satzkandidaten, sobald du eine Pause machst.

Die Seite „Statistik“ zeigt zudem die Eingabemenge von heute, der letzten 7 Tage und insgesamt sowie die Vokabelaufzeichnungen der Lernsprache. Einzelheiten findest du unter [Eingabe](https://qingjian.app/docs/input), [Übersetzungen und neue Wörter](https://qingjian.app/docs/learning/translation), [Lokale Statistik](https://qingjian.app/docs/learning/statistics) und [Tasten und Tastenkürzel](https://qingjian.app/docs/getting-started/keys).

## Daten und Datenschutz

Pinyin-Umwandlung, Wörterbuchsuche, das lokale Modell und das Lernen deiner Tippgewohnheiten laufen auf deinem Gerät. Auch Eingabemenge und Vokabelstatistik werden nur auf diesem Rechner gespeichert und nie hochgeladen; Qingjian braucht kein Konto. Eingabeprotokoll und Statistik werden getrennt gespeichert: Das Protokoll lässt sich in den Einstellungen abschalten oder löschen, ohne die Statistik zu berühren. Die Update-Prüfung ruft die Versionsliste von der Website ab und lässt sich in den Einstellungen abschalten.

Die optionale **Cloud-Vorhersage ist standardmäßig aus**. Schaltest du sie ein, sendet Qingjian deine aktuelle Eingabe und den umgebenden Text direkt an den KI-Anbieter, den du selbst eingerichtet hast, um Kandidaten oder Satzvervollständigungen zu erhalten; die Anfragen laufen nicht über Server von Qingjian. Was gesendet und was auf diesem Rechner gespeichert wird, steht unter [Datenschutz](https://qingjian.app/docs/help/privacy).

## Dokumentation, Feedback und Mitwirken

- [Benutzerdokumentation](https://qingjian.app/docs): Installation, Einstellungen, Eingabe, Deinstallation und häufige Fragen.
- [Ein Problem melden oder einen Vorschlag machen](https://github.com/qingjian-team/qingjian/issues/new/choose); du kannst auch der [QQ-Betatestgruppe](https://qm.qq.com/q/jBvn2gGTxm) beitreten.
- Du möchtest mitentwickeln? Fang mit der [Entwicklerdokumentation](../) und den [Beitragsrichtlinien](../contributing.md) an.

Qingjian ist über den [offiziellen Kanal](https://qingjian.app/download) kostenlos erhältlich. Der Code steht unter der Lizenz [GPL-3.0-or-later](../../LICENSE); Projektname und Logo sind nicht von der Code-Lizenz umfasst. Mitgelieferte Daten haben eigene Quellen und Lizenzen, siehe die [Liste der Datenquellen](../design/landscape.md).

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="GitHub-Stern-Verlauf von Qingjian" width="600">
    </picture>
  </a>
</p>

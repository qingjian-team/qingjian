<p align="center"><a href="../../README.md">简体中文</a> | <a href="README.zh-Hant.md">繁體中文</a> | <a href="README.en.md">English</a> | <a href="README.ja.md">日本語</a> | <b>Español</b> | <a href="README.vi.md">Tiếng Việt</a> | <a href="README.de.md">Deutsch</a></p>

<p align="center">
  <img src="../../assets/icon/qingjian-mark.svg" alt="Icono de tablilla de bambú de Qingjian" height="108">
</p>

<h1 align="center">青简 Qingjian</h1>

<p align="center"><strong>Escribe como siempre y, de paso, aprende una palabra.</strong></p>

<p align="center">
  <a href="https://qingjian.app/download"><img src="https://img.shields.io/github/v/release/qingjian-team/qingjian?label=stable" alt="stable release"></a>
  <a href="https://github.com/qingjian-team/qingjian/releases"><img src="https://img.shields.io/github/downloads/qingjian-team/qingjian/total?label=downloads" alt="GitHub downloads"></a>
  <a href="https://github.com/qingjian-team/qingjian/actions/workflows/ci.yml?query=branch%3Amain"><img src="https://img.shields.io/github/actions/workflow/status/qingjian-team/qingjian/ci.yml?branch=main&amp;label=CI" alt="CI"></a>
  <a href="https://github.com/qingjian-team/qingjian/stargazers"><img src="https://img.shields.io/github/stars/qingjian-team/qingjian?style=flat&amp;label=Stars" alt="GitHub Stars"></a>
  <a href="../../LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0--or--later-blue" alt="License: GPL-3.0-or-later"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/macOS-13%2B-blue?logo=apple&amp;logoColor=white" alt="macOS 13+"></a>
  <a href="https://qingjian.app/docs/getting-started/install"><img src="https://img.shields.io/badge/Windows-10%2F11-blue?logo=data:image/svg%2bxml;base64,PHN2ZyB4bWxucz0iaHR0cDovL3d3dy53My5vcmcvMjAwMC9zdmciIHZpZXdCb3g9IjAgMCAyNCAyNCI%2BPHBhdGggZmlsbD0id2hpdGUiIGQ9Ik0wIDBoMTEuNHYxMS40SDB6TTEyLjYgMEgyNHYxMS40SDEyLjZ6TTAgMTIuNmgxMS40VjI0SDB6TTEyLjYgMTIuNkgyNFYyNEgxMi42eiIvPjwvc3ZnPg==" alt="Windows 10/11"></a>
  <a href="https://qingjian.app/docs/platforms/linux"><img src="https://img.shields.io/badge/Linux-Fcitx4%20%2F%205%20manual-lightgrey?logo=linux&amp;logoColor=white" alt="Linux: Fcitx4 / 5, inicio manual"></a>
  <a href="https://qingjian.app/docs/platforms/android"><img src="https://img.shields.io/badge/Android-8.0%2B%20preview-lightgrey?logo=android&amp;logoColor=white" alt="Android 8.0+: versión preliminar"></a>
</p>

Qingjian es un método de entrada. Escribes como lo harías normalmente: introduces pinyin, eliges un candidato, escribes una frase completa. Una traducción junto a cada candidato hace que el aprendizaje de idiomas ocurra de forma natural mientras escribes. La traducción es siempre información auxiliar; nunca ocupa el lugar del texto que estás escribiendo.

https://github.com/user-attachments/assets/d145fde9-a641-4543-8b15-dd7a2685de3d

El vídeo muestra en macOS la entrada de frases completas, la reordenación de candidatos y las traducciones de los candidatos.

## Descarga y primeros pasos

- **macOS, Windows**: [descarga Qingjian](https://qingjian.app/download); los pasos de instalación están en la [documentación de usuario](https://qingjian.app/docs/getting-started/install).
- **Linux**: Fcitx5 y Fcitx4 se instalan desde el código fuente y usan el panel de candidatos predeterminado del sistema; por ahora el servicio en segundo plano se inicia a mano. Consulta la [instalación en Linux](https://qingjian.app/docs/platforms/linux).
- **Android**: versión preliminar, con menos funciones que las de escritorio; descarga el APK arm64 en [GitHub Releases](https://github.com/qingjian-team/qingjian/releases/latest). Para activarlo consulta [Android](https://qingjian.app/docs/platforms/android) y para compilarlo desde el código fuente, [apps/android](../../apps/android/README.md).

**Canales oficiales de publicación**: por ahora solo son estos; un «Qingjian» de cualquier otro sitio no es una publicación oficial.

- [La página de descargas del sitio web](https://qingjian.app/download)
- [GitHub Releases](https://github.com/qingjian-team/qingjian/releases)
- El enlace de Quark Drive que aparece en el registro de cambios de cada versión

Qingjian todavía no está publicado en ningún gestor de paquetes.

Las versiones de macOS y Windows siguen en fase beta. Tras instalarlo, selecciona Qingjian, elige el idioma que quieres aprender en «Preferencias / Ajustes → General» y empieza a escribir. Si es la primera vez, empieza por [Primera entrada](https://qingjian.app/docs/getting-started/first-input).

## Lo que ves al escribir

```text
1  开发        desarrollar
2  编程        programar
3  架构        arquitectura
```

Junto a los candidatos solo se muestra un idioma de aprendizaje a la vez. Hoy puedes elegir inglés, japonés, español, vietnamita o alemán, o desactivar las traducciones. Qingjian también admite entrada de frases completas, pinyin abreviado, corrección ortográfica, pinyin doble, Wubi y más; un modelo de frases local ajusta los candidatos de la frase cuando haces una pausa.

La página «Estadísticas» muestra además el volumen escrito hoy, en los últimos 7 días y en total, y el registro de vocabulario del idioma que estás aprendiendo. Para más detalles, consulta [Entrada](https://qingjian.app/docs/input), [Traducciones y palabras nuevas](https://qingjian.app/docs/learning/translation), [Estadísticas locales](https://qingjian.app/docs/learning/statistics) y [Teclas y atajos](https://qingjian.app/docs/getting-started/keys).

## Datos y privacidad

La conversión de pinyin, la consulta de diccionarios, el modelo local y el aprendizaje de tus hábitos de escritura se realizan en tu dispositivo. El volumen escrito y las estadísticas de vocabulario también se guardan solo en este equipo y nunca se suben; Qingjian no necesita cuenta. El registro de entrada y las estadísticas se guardan por separado: el registro se puede desactivar o borrar en Ajustes sin afectar a las estadísticas. La búsqueda de actualizaciones pide la lista de versiones al sitio web y se puede desactivar en Ajustes.

La **predicción en la nube**, opcional, **está desactivada por defecto**. Si la activas, Qingjian envía tu entrada actual y el texto cercano directamente al proveedor de IA que tú mismo configures, para obtener candidatos o completar frases; las solicitudes no pasan por los servidores de Qingjian. Qué se envía y qué se guarda en este equipo se explica en [Privacidad](https://qingjian.app/docs/help/privacy).

## Documentación, comentarios y cómo contribuir

- [Documentación de usuario](https://qingjian.app/docs): instalación, ajustes, entrada, desinstalación y preguntas frecuentes.
- [Informa de un problema o envía una sugerencia](https://github.com/qingjian-team/qingjian/issues/new/choose); también puedes unirte al [grupo de pruebas beta en QQ](https://qm.qq.com/q/jBvn2gGTxm).
- ¿Quieres contribuir? Empieza por la [documentación para desarrolladores](../) y la [guía de contribución](../contributing.md).

Qingjian es gratuito a través del [canal oficial](https://qingjian.app/download). El código se distribuye bajo la licencia [GPL-3.0-or-later](../../LICENSE); el nombre y el logotipo del proyecto no están cubiertos por la licencia del código. Los datos incluidos tienen sus propias fuentes y licencias; consulta la [lista de procedencia de datos](../design/landscape.md).

## Star History

<p align="center">
  <a href="https://www.star-history.com/#qingjian-team/qingjian&Date">
    <picture>
      <source media="(prefers-color-scheme: dark)" srcset="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date&theme=dark">
      <img src="https://api.star-history.com/svg?repos=qingjian-team/qingjian&type=Date" alt="Evolución de estrellas de Qingjian en GitHub" width="600">
    </picture>
  </a>
</p>

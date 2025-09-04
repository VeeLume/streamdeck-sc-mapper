<!-- Improved compatibility of back to top link: See: https://github.com/othneildrew/Best-README-Template/pull/73 -->
<a id="readme-top"></a>
<!--
*** Thanks for checking out the Best-README-Template. If you have a suggestion
*** that would make this better, please fork the repo and create a pull request
*** or simply open an issue with the tag "enhancement".
*** Don't forget to give the project a star!
*** Thanks again! Now go create something AMAZING! :D
-->



<!-- PROJECT SHIELDS -->
<!--
*** I'm using markdown "reference style" links for readability.
*** Reference links are enclosed in brackets [ ] instead of parentheses ( ).
*** See the bottom of this document for the declaration of the reference variables
*** for contributors-url, forks-url, etc. This is an optional, concise syntax you may use.
*** https://www.markdownguide.org/basic-syntax/#reference-style-links
-->
[![Contributors][contributors-shield]][contributors-url]
[![Forks][forks-shield]][forks-url]
[![Stargazers][stars-shield]][stars-url]
[![Issues][issues-shield]][issues-url]
[![project_license][license-shield]][license-url]
<!-- [![LinkedIn][linkedin-shield]][linkedin-url] -->



<!-- PROJECT LOGO -->
<br />
<div align="center">
  <!-- <a href="https://github.com/veelume/streamdeck-sc-mapper">
    <img src="images/logo.png" alt="Logo" width="80" height="80">
  </a> -->

<h3 align="center">Stream Deck SC Mapper</h3>

  <p align="center">
    Helps manage Star Citizen keybinds
    <br />
    <!-- <a href="https://github.com/veelume/streamdeck-sc-mapper"><strong>Explore the docs »</strong></a> -->
    <!-- <br /> -->
    <!-- <br /> -->
    <a href="https://github.com/veelume/streamdeck-sc-mapper/releases/latest">Download</a>
    &middot;
    <a href="https://github.com/veelume/streamdeck-sc-mapper/issues/new?labels=bug&template=bug-report---.md">Report Bug</a>
    &middot;
    <a href="https://github.com/veelume/streamdeck-sc-mapper/issues/new?labels=enhancement&template=feature-request---.md">Request Feature</a>
  </p>
</div>



<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#recommended-install">Recommended Install</a></li>
        <li><a href="#optional-cli-tool">Optional CLI Tool</a></li>
        <li><a href="#advanced-build">Advanced Build</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#contributing">Contributing</a></li>
    <li><a href="#license">License</a></li>
    <li><a href="#contact">Contact</a></li>
    <li><a href="#support">Support</a></li>
    <!-- <li><a href="#acknowledgments">Acknowledgments</a></li> -->
  </ol>
</details>



<!-- ABOUT THE PROJECT -->
## About The Project

<!-- [![Product Name Screen Shot][product-screenshot]](https://example.com) -->

**Stream Deck SC Mapper** helps manage Star Citizen keybinds:

- Stream Deck plugin — browse, trigger, and manage actions directly from your deck.
- CLI tool (`scmap-gen`) — generate `mappings.xml` from the game’s `defaultProfile.xml` and optional custom profile.
- Core crate — parsing, bind generation, translations, and profile I/O shared by both.

**Workspace layout:**

```
crates/
  core/     # streamdeck-sc-core: shared logic
  plugin/   # streamdeck-sc-mapper: Stream Deck plugin
  cli/      # scmap-gen: CLI tool
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>



### Built With

- [Rust](https://www.rust-lang.org/)
- [Elgato Stream Deck SDK](https://developer.elgato.com/documentation/stream-deck/sdk/overview/)

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- GETTING STARTED -->
## Getting Started

There are two ways to use this project:
- **Normal users** — install the Stream Deck plugin from Releases (recommended).
- **Advanced users** — use the CLI tool (`scmap-gen`) or build from source.

---

### Recommended Install

1. Go to the [Releases page](https://github.com/veelume/streamdeck-sc-mapper/releases).
2. Download the latest `.streamDeckPlugin` file (e.g. `icu.veelume.sc-mapper.streamDeckPlugin`).
3. Double-click it — the Stream Deck app will automatically install or update the plugin.
4. Restart the Stream Deck software if it doesn’t appear right away.
5. Open the Stream Deck app → look for the **SC Mapper** category → drag actions onto your keys.

---

### Optional CLI Tool

The CLI lets you generate or rebuild Star Citizen keybinding profiles outside the plugin.

**Windows example:**

1. Download `scmap-gen.exe` from the release assets.
2. Open **Command Prompt** (Win+R → `cmd`).
3. Run:

```sh
scmap-gen.exe --default "./defaultProfile.xml" --include-custom
```

Options:
- `--default` -> required, points to the `defaultProfile.xml` extracted from the game files
You find the current shipped one here [here](https://github.com/VeeLume/streamdeck-sc-mapper/blob/main/icu.veelume.sc-mapper.sdPlugin/defaultProfile.xml) (i try to keep that up to date until i have extraction solution ready)
- `--include-custom` -> also merges you current keybinds
- Result: generates `mappings-generated.xml` profile with missing binds filled in

To see all possible options run:
```sh
scmap-gen.exe --default "./defaultProfile.xml" --help
```

### Advanced Build

If you want to develop or build your own binaries:
```sh
git clone https://github.com/veelume/streamdeck-sc-mapper.git
cd streamdeck-sc-mapper
cargo build --release
```

The built plugin will be under `target/release/`, and you can repackage or link it with the Elgato CLI.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE EXAMPLES -->
## Usage

### CLI

```sh
scmap-gen --default "path/to/defaultProfile.xml" --include-custom
```

Generates `mappings-generated.xml` with missing binds filled in.

### Plugin

After installation, add SC Mapper actions to your Stream Deck.

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- ROADMAP -->
## Roadmap

- [ ] Automatically extract the `defaultProfile.xml` and `globals.ini` from the game files
- [ ] Power setup action
- [ ] MFD setup action
- [ ] Expand cli options
- [ ] Profile export/import UI

See the [open issues](https://github.com/veelume/streamdeck-sc-mapper/issues) for a full list of proposed features (and known issues).

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTRIBUTING -->
## Contributing

Contributions are what make the open source community such an amazing place to learn, inspire, and create. Any contributions you make are **greatly appreciated**.

If you have a suggestion that would make this better, please fork the repo and create a pull request. You can also simply open an issue with the tag "enhancement".
Don't forget to give the project a star! Thanks again!

1. Fork the Project
2. Create your Feature Branch (`git checkout -b feature/AmazingFeature`)
3. Commit your Changes (`git commit -m 'Add some AmazingFeature'`)
4. Push to the Branch (`git push origin feature/AmazingFeature`)
5. Open a Pull Request

<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Top contributors:

<a href="https://github.com/veelume/streamdeck-sc-mapper/graphs/contributors">
  <img src="https://contrib.rocks/image?repo=veelume/streamdeck-sc-mapper" alt="contrib.rocks image" />
</a>



<!-- LICENSE -->
## License

This project is licensed under either of
- MIT License ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)
- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
at your option.

<p align="right">(<a href="#readme-top">back to top</a>)</p>



<!-- CONTACT -->
## Contact

<!-- Your Name - [@twitter_handle](https://twitter.com/twitter_handle) - email@email_client.com -->

Project Link: [https://github.com/veelume/streamdeck-sc-mapper](https://github.com/veelume/streamdeck-sc-mapper)

<p align="right">(<a href="#readme-top">back to top</a>)</p>


## Support
- Open an [Issue](https://github.com/veelume/streamdeck-sc-mapper/issues) for bugs or feature requests
- [Discussions](https://github.com/VeeLume/streamdeck-sc-mapper/discussions) for questions or any other input


<!-- ACKNOWLEDGMENTS -->
<!-- ## Acknowledgments

* []()
* []()
* []()

<p align="right">(<a href="#readme-top">back to top</a>)</p> -->



<!-- MARKDOWN LINKS & IMAGES -->
<!-- https://www.markdownguide.org/basic-syntax/#reference-style-links -->
[contributors-shield]: https://img.shields.io/github/contributors/veelume/streamdeck-sc-mapper.svg?style=for-the-badge
[contributors-url]: https://github.com/veelume/streamdeck-sc-mapper/graphs/contributors
[forks-shield]: https://img.shields.io/github/forks/veelume/streamdeck-sc-mapper.svg?style=for-the-badge
[forks-url]: https://github.com/veelume/streamdeck-sc-mapper/network/members
[stars-shield]: https://img.shields.io/github/stars/veelume/streamdeck-sc-mapper.svg?style=for-the-badge
[stars-url]: https://github.com/veelume/streamdeck-sc-mapper/stargazers
[issues-shield]: https://img.shields.io/github/issues/veelume/streamdeck-sc-mapper.svg?style=for-the-badge
[issues-url]: https://github.com/veelume/streamdeck-sc-mapper/issues
[license-shield]: https://img.shields.io/github/license/veelume/streamdeck-sc-mapper.svg?style=for-the-badge
[license-url]: https://github.com/VeeLume/streamdeck-sc-mapper/blob/main/LICENSE
[linkedin-shield]: https://img.shields.io/badge/-LinkedIn-black.svg?style=for-the-badge&logo=linkedin&colorB=555
[linkedin-url]: https://linkedin.com/in/linkedin_username
[product-screenshot]: images/screenshot.png
[Next.js]: https://img.shields.io/badge/next.js-000000?style=for-the-badge&logo=nextdotjs&logoColor=white
[Next-url]: https://nextjs.org/
[React.js]: https://img.shields.io/badge/React-20232A?style=for-the-badge&logo=react&logoColor=61DAFB
[React-url]: https://reactjs.org/
[Vue.js]: https://img.shields.io/badge/Vue.js-35495E?style=for-the-badge&logo=vuedotjs&logoColor=4FC08D
[Vue-url]: https://vuejs.org/
[Angular.io]: https://img.shields.io/badge/Angular-DD0031?style=for-the-badge&logo=angular&logoColor=white
[Angular-url]: https://angular.io/
[Svelte.dev]: https://img.shields.io/badge/Svelte-4A4A55?style=for-the-badge&logo=svelte&logoColor=FF3E00
[Svelte-url]: https://svelte.dev/
[Laravel.com]: https://img.shields.io/badge/Laravel-FF2D20?style=for-the-badge&logo=laravel&logoColor=white
[Laravel-url]: https://laravel.com
[Bootstrap.com]: https://img.shields.io/badge/Bootstrap-563D7C?style=for-the-badge&logo=bootstrap&logoColor=white
[Bootstrap-url]: https://getbootstrap.com
[JQuery.com]: https://img.shields.io/badge/jQuery-0769AD?style=for-the-badge&logo=jquery&logoColor=white
[JQuery-url]: https://jquery.com

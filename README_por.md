<p align="center">
  <img src="images/EH3D_BANNER.svg" alt="EH3D-STENCILCAM banner" width="100%">
</p>

# <img src="images/brand/icon-64.png" alt="" width="40" height="40" align="absmiddle"> EH3D-STENCILCAM
<p align="center"><a href="README.md">🇺🇸 English</a> | <a href="README_spa.md">🇪🇸 Español</a> | <a href="README_fra.md">🇫🇷 Français</a> | <a href="README_ita.md">🇮🇹 Italiano</a> | <a href="README_deu.md">🇩🇪 Deutsch</a> | <a href="README_zho.md">🇨🇳 中文</a> | <a href="README_jpn.md">🇯🇵 日本語</a> | 🇵🇹 <b>Português</b></p>

### Stencils de pasta de solda a partir da camada de pasta: aberturas, verificação de proporções e trajetórias de corte

<p align="center">
  <img src="https://img.shields.io/badge/License-GPL%203.0--or--later-blue.svg" alt="GPL 3.0 or later">
  <img src="https://img.shields.io/badge/Stack-Rust-DEA584.svg" alt="Rust">
  <img src="https://img.shields.io/badge/Languages-8-00E5FF.svg" alt="Languages">
  <img src="https://img.shields.io/badge/Version-0.0.4-0f6cbd.svg" alt="Version">
  <img src="https://img.shields.io/badge/Maturity-scaffolding-FFB020.svg" alt="Maturity">
</p>

---

**Verificação de honestidade - o que funciona hoje:** uma biblioteca Rust e um programa de linha de comandos (`eh3d-stencilcam`) com 117 testes que correm num computador: transforma os pads de uma camada de pasta em aberturas de stencil (reduzidas, arredondadas, compensadas pelo kerf do laser e verificadas para a libertação da pasta) e escreve-as como G-code ou HP-GL. Tem uma janela (compilada com `--features gui`) que abre uma camada de pasta e desenha os pads e as trajetórias de corte, mas que ainda não foi vista num ecrã; não foi experimentado numa máquina e não foi verificado com uma coleção de ficheiros reais.

---

> **Estado de segurança:** este programa não controla nenhuma máquina e não é um sistema de segurança. O que produz deve ser verificado por uma pessoa antes de se executar algo numa máquina; nada disto correu em hardware real.


## 📋 Num relance

| | |
|---|---|
| Versão | `0.0.4` |
| Maturidade | Estrutura inicial |
| Testes | 117 |
| Tecnologias | Rust |
| Em hardware real | Não experimentado: nada disto correu numa máquina |

**Ir para:** [Visão geral](#-visão-geral) · [Arquitetura](#-arquitetura) · [Início rápido](#-início-rápido) · [Testes](#-testes) · [Estrutura](#-estrutura-do-repositório) · [Relacionados](#-projetos-relacionados)

## 🚦 Estado do projeto

Cada linha é uma parte deste repositório e a luz diz até onde chega hoje. É escrita com o que o repositório consegue demonstrar (testes, compilações, execuções), não com o que está previsto.

🟢 Feito e verificado · 🟡 Funciona, com limites conhecidos · 🟠 Feito a meio ou ainda por verificar · 🔴 Por fazer ou a não funcionar

| Área | Estado | O que significa aqui |
|---|---|---|
| Núcleo | 🟡 | transforma os pads de uma camada de pasta em aberturas de stencil (reduzidas, arredondadas, compensadas pelo kerf do laser e verificadas para a libertação da pasta) e escreve-as como G-code ou HP-GL; testado num computador com entradas normais e danificadas, não com uma coleção de ficheiros reais. |
| Linha de comandos | 🟡 | `eh3d-stencilcam` executa as mesmas funções da biblioteca e imprime o resultado; cada erro volta como mensagem e código de saída diferente de zero. |
| Testes | 🟡 | 117 testes passam (`cargo test`), com casos normais, casos-limite e entradas danificadas. |
| Segurança | 🟡 | Rust seguro e simples (`forbid(unsafe_code)`): nada entra em pânico com os ficheiros recebidos e um valor mau é um erro com o seu motivo; sem revisão independente. |
| Janela | 🟠 | Escrita (egui, `--features gui`): abrir ou largar uma camada de pasta, escolher o material, editar os parâmetros, ver os pads e as trajetórias de corte (a vermelho as aberturas que não libertarão a pasta), zoom e deslocamento, exportar; a sua lógica está testada, mas ainda não foi compilada nem vista num ecrã. |
| Hardware real | 🔴 | Não experimentado: nada disto foi executado numa máquina. |
| Documentação | 🟡 | README em oito idiomas, o registo de alterações e as duas imagens desenhadas a partir de `docs/`. |

## 🎯 Visão geral

* **As aberturas** (`src/openings.rs`): cada pad flasheado na camada de pasta (retangular, redondo ou oval, em milímetros sejam quais forem as unidades do ficheiro) é uma abertura; é reduzida pela percentagem do material e a linha de corte fica mais pequena pelo kerf do laser para que o furo saia do tamanho pedido. Um stencil para a face inferior pode ser espelhado.
* **As proporções** (`src/openings.rs`): a relação de aspeto (o lado pequeno sobre a espessura da folha, pelo menos 1,5) e a relação de área (a área da abertura sobre a área das suas paredes, pelo menos 0,66) dizem se a pasta sairá da abertura; uma abertura que falha é indicada com a sua posição e `convert` recusa-se a escrever até ser vista (ou `--force`).
* **As trajetórias** (`src/output.rs`, `src/profile.rs`): retângulos arredondados, círculos e ovais como trajetórias fechadas pela ordem de um percurso ao mais próximo; G-code para um laser de potência constante, desligado nas deslocações, ou HP-GL a 40 unidades por milímetro; perfis de Mylar, Kapton e aço com espessura, redução, raio de canto e kerf como pontos de partida.
* **Não se escreve um stencil incompleto:** `convert` recusa-se a escrever o ficheiro quando a camada tem pads, pistas ou regiões que não podem ser aberturas (polígonos, macros, regiões), a não ser com `--force`.
* **A camada é lida a partir dos seus bytes:** `--max-bytes N` recusa um ficheiro acima do limite (256 MiB por omissão); o texto que não é UTF-8 é lido como Latin-1 com um aviso. Os avisos e erros do leitor são impressos com o seu código estável e `--hide CODE[,CODE]` omite os avisos desses códigos; um erro do leitor conta como uma parte perdida da camada, por isso `convert` recusa-se a não ser com `--force`. As aberturas vêm da conversão das aberturas de EH3D-GERBERVIEWER, e um pad poligonal é cortado como um círculo (com uma nota, por isso `convert` também o recusa a não ser com `--force`). `--arc-tolerance MM` define o rigor dos arcos, e a recusa de um ficheiro demasiado grande é escrita na língua do sistema.

## 🧩 Papel no EH3D

EH3D-STENCILCAM é um programa da EH3D Fabrication Suite. Funciona como biblioteca e linha de comandos e, compilado com `--features gui`, como janela com o aspeto que os outros programas da suíte partilham. Lê e escreve ficheiros simples, por isso pode ser usado sozinho ou entre os outros programas.

## 🏛️ Arquitetura

<p align="center">
  <img src="images/architecture.svg" alt="Arquitetura de EH3D-STENCILCAM: quem o usa, o que contém e de que depende" width="100%">
</p>


Como as partes deste repositório se encaixam: quem o usa, o que contém e de que depende. A imagem é desenhada a partir de `docs/architecture.json` e verificada com o código.

## ✨ Funcionalidades

* **Biblioteca e linha de comandos:** cada comando de `eh3d-stencilcam` é uma função da biblioteca, por isso outro programa da suíte pode chamá-la sem lançar um processo.
* **Linha de comandos estrita:** uma opção mal escrita (`--powr`) ou uma palavra a mais é um erro que a nomeia, não algo que se ignora; caso contrário o programa faria a sua saída com os valores por omissão.
* **Testes com entradas danificadas:** os testes também dão ficheiros vazios, lixo, números enormes e ficheiros cortados, e nenhum pode entrar em pânico.
* **Sem adivinhar:** um valor sem sentido é um erro que diz qual e porquê, nunca um resultado que apenas parece certo.
* **Sem código unsafe:** `#![forbid(unsafe_code)]` na biblioteca e no programa.
* **O G-code leva o cabeçalho padrão e uma ordem mais rápida:** o ficheiro começa com linhas de comentário do gerador, das unidades e da camada com o seu SHA-256, um retângulo de cantos vivos não tem pontos repetidos e a ordem dos cortes encontra-se com uma grelha, de modo que cem mil aberturas ficam ordenadas num instante.
* **Uma espera depois de ligar o laser:** `--laser-wait SEGUNDOS` escreve uma espera `G4` depois de cada `M3` para que o laser esteja à sua potência antes de a cabeça se mover (0 por omissão: sem espera); os testes voltam a ler o G-code com o leitor da suite e não encontram nenhum aviso nem nenhum corte com o laser desligado.

## 🔒 Segurança e proteção

| Ameaça | Defesa |
|---|---|
| Um ficheiro danificado, cortado ou vazio | sem pânico: os testes cortam uma entrada válida em cada posição e dão lixo |
| Um número sem sentido (negativo, não finito, absurdamente grande) | recusado com um erro que nomeia o valor; nada é arredondado para algo plausível |
| Uso inseguro de memória | Rust seguro e simples com `forbid(unsafe_code)` |

**O que não é defendido:** um resultado produzido sem erros não é um resultado verificado. Nada aqui conhece os limites de uma máquina e o programa não teve revisão independente. Comunique uma vulnerabilidade em privado (ver [SECURITY.md](SECURITY.md)).

## 🔌 Interfaces

A biblioteca é `eh3d_stencilcam` (`use eh3d_stencilcam::...`); a linha de comandos é `eh3d-stencilcam`. Os comandos são estes; cada um imprime o resultado na saída padrão e comunica um erro na saída de erros com um código de saída diferente de zero. Não há acesso à rede.

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── layer.rs                                 lê uma camada a partir de bytes e mostra o que o leitor diz pelo seu código
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── openings.rs                              uma parte da lógica
│   ├── output.rs                                uma parte da lógica
│   ├── profile.rs                               uma parte da lógica
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: camada, material, parâmetros, cortes, exportação e estado
├── tests/
│   ├── reading.rs                               testes de leitura de ficheiros: o limite de tamanho, texto que não é UTF-8, avisos por código
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── stencil.rs                               testes da biblioteca e da linha de comandos
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   └── window_cli.rs                            testes de que uma compilação sem a funcionalidade gui continua a ser só linha de comandos
├── examples/
│   └── paste.gtp                                um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── rust-toolchain.toml                          a toolchain de Rust fixada
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

## ⚙️ Configuração

Ainda não há ficheiro de configuração: o que a pessoa escolhe é dado como argumentos da linha de comandos (ver Interfaces) e cada valor por omissão está escrito na ajuda do comando (`eh3d-stencilcam help`). Os limites fixos são constantes com nome no código, cobertas por testes.

## 🚀 Início rápido

Precisa de uma cadeia de ferramentas Rust estável (1.75 ou mais recente).

```bash
cargo build --release
cargo test
cargo run --release -- check examples/paste.gtp
cargo run --release --features gui   # the window
```

## 🧪 Testes

O conjunto tem 117 testes (`cargo test`, executados em Windows e Linux no CI). Verificam os casos normais, os casos-limite e as entradas danificadas: ficheiros vazios, lixo e ficheiros cortados nunca provocam pânico. O CI executa ainda `cargo fmt --check`, `cargo clippy` com avisos como erros e `python -I tools/ci_validate.py`.

## 🏗️ Compilação e publicação

A versão está em `Cargo.toml` e em `eh3d.project.json`; cada versão tem a sua entrada em [CHANGELOG.md](CHANGELOG.md). `build.bat` / `build.sh` compilam o programa, `test.bat` / `test.sh` executam os testes e `run.bat` / `run.sh` executam-no com um exemplo. As versões são publicadas no GitHub.

## 📂 Estrutura do repositório

<p align="center">
  <img src="images/repository-structure.svg" alt="Estrutura do repositório EH3D-STENCILCAM" width="100%">
</p>

<details>
<summary>Versão em texto da árvore</summary>

```text
EH3D-STENCILCAM/
├── src/
│   ├── browse.rs                                a lista de ficheiros da janela que abre um ficheiro
│   ├── cli_args.rs                              os argumentos da linha de comandos, o mesmo ficheiro em cada utilitário pequeno
│   ├── gui.rs                                   a janela (egui, com a funcionalidade gui): desenha o estado e transmite o que a pessoa faz
│   ├── lang.rs                                  as palavras da janela nos oito idiomas
│   ├── layer.rs                                 lê uma camada a partir de bytes e mostra o que o leitor diz pelo seu código
│   ├── lib.rs                                   a biblioteca: o que cada comando usa
│   ├── main.rs                                  a linha de comandos
│   ├── openings.rs                              uma parte da lógica
│   ├── output.rs                                uma parte da lógica
│   ├── profile.rs                               uma parte da lógica
│   ├── viewport.rs                              zoom e deslocamento da imagem, em números simples
│   └── window.rs                                o estado da janela sem ecrã: camada, material, parâmetros, cortes, exportação e estado
├── tests/
│   ├── reading.rs                               testes de leitura de ficheiros: o limite de tamanho, texto que não é UTF-8, avisos por código
│   ├── readme.rs                                testes que executam o que o README e os scripts run prometem
│   ├── stencil.rs                               testes da biblioteca e da linha de comandos
│   ├── strict.rs                                testes da linha de comandos: opções e palavras que não conhece
│   └── window_cli.rs                            testes de que uma compilação sem a funcionalidade gui continua a ser só linha de comandos
├── examples/
│   └── paste.gtp                                um exemplo para experimentar o programa
├── docs/
│   ├── architecture.json                        dados da imagem de Arquitetura
│   └── structure.json                           dados da imagem de Estrutura
├── tools/                                       verificação da CI e do README e ferramentas de imagens
├── images/                                      banner e as imagens de Arquitetura e Estrutura do repositório
├── .github/                                     fluxos de CI e de publicação, modelos de issues e pull requests
├── eh3d.project.json                            manifesto: versão, papel, maturidade e comando de compilação
├── Cargo.toml Cargo.lock                        o workspace e as suas dependências fixadas
├── rust-toolchain.toml                          a toolchain de Rust fixada
├── CHANGELOG.md                                 o que mudou em cada versão
├── SECURITY.md SUPPORT.md                       como comunicar uma vulnerabilidade e onde perguntar
└── CONTRIBUTING.md CODE_OF_CONDUCT.md LICENSE   como contribuir, o código de conduta e a licença
```

</details>

## 🗺️ Roteiro e limites conhecidos

**Limites conhecidos hoje**

* A janela ainda não foi vista num ecrã e não mostra as relações de cada abertura: isso faz o `check`.
* Não foi experimentado numa máquina nem com uma coleção de ficheiros reais e não teve revisão independente.
* Os pads de macro e as pistas e regiões da camada de pasta não se tornam aberturas (um pad poligonal é cortado como um círculo), os perfis dos materiais são pontos de partida e não há corte em várias passagens para uma folha de aço.

**A seguir**

* Experimentá-lo com uma camada de pasta real e um laser e ler os perfis do armazém de ferramentas.
* Ver a janela num ecrã e completá-la: os perfis do armazém de ferramentas e uma vista das relações de pasta de cada abertura.
* Experimentá-lo com ficheiros reais e com os outros programas da suíte.

## 🌍 Idiomas

O README e os textos existem em inglês, espanhol, francês, italiano, alemão, chinês, japonês e português europeu.

## 🔗 Projetos relacionados

* [EH3D-CORE-LIBS](https://github.com/JuanenRac/EH3D-CORE-LIBS): os leitores de Gerber, Excellon e G-code
* [EH3D-PHOENIX-LASER](https://github.com/JuanenRac/EH3D-PHOENIX-LASER): a gravação e corte a laser
* [EH3D-GERBERVIEWER](https://github.com/JuanenRac/EH3D-GERBERVIEWER): o visualizador de camadas Gerber
* [EH3D-TOOL-MANAGER](https://github.com/JuanenRac/EH3D-TOOL-MANAGER): o armazém de ferramentas partilhado pelos programas CAM
* [EH3D-CAM-DEBUGGER](https://github.com/JuanenRac/EH3D-CAM-DEBUGGER): a verificação do G-code antes de chegar a uma máquina

## 📚 Documentação

* [Registo de alterações](CHANGELOG.md)
* [Dados de arquitetura e estrutura](docs/architecture.json): os dados a partir dos quais as duas imagens são desenhadas.

## 🤝 Contribuir e comunidade

* Antes de um pull request execute `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test` e `python -I tools/ci_validate.py`: tudo tem de passar. Veja [CONTRIBUTING.md](CONTRIBUTING.md) e o [código de conduta](CODE_OF_CONDUCT.md).
* O README existe em oito idiomas: uma alteração ao texto faz-se nos oito ficheiros, e as duas imagens são desenhadas a partir de `docs/architecture.json` e `docs/structure.json` por `tools/make_architecture.py` e `tools/make_structure.py`.
* Comunique uma vulnerabilidade em privado: [SECURITY.md](SECURITY.md). Perguntas, ideias e relatos: electrohobby3d@gmail.com ([SUPPORT.md](SUPPORT.md)).

## 👤 AUTOR

**JuanenRac (Electro Hobby 3D)** · electrohobby3d@gmail.com

## 📜 LICENÇA

Software: GPL-3.0-or-later. Ver [LICENSE](LICENSE). Os projetos de hardware da suite usam a CERN OHL-S.

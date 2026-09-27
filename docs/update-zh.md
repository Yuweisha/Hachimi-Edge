# 上游更新后怎么手动更新

## 仓库关系

| remote | 指向 | 用途 |
| --- | --- | --- |
| `origin` | `Yuweisha/Hachimi-Edge`（你的私有仓库） | 日常提交、推送 |
| `upstream` | `kairusds/Hachimi-Edge`（上游） | 只用来拉更新，**push 已禁用**（防误推） |

你的两个提交（全局角色替换 + 下拉选择）直接建立在上游历史之上，所以上游更新时可以正常 merge，不需要重建仓库。

## 方法一：一条命令（推荐）

在 Git Bash 里：

```bash
cd C:/Users/Schwarz/projects/Hachimi-Edge
bash tools/update-from-upstream.sh          # 只同步代码
bash tools/update-from-upstream.sh --build  # 同步 + 构建出 hachimi.dll
```

脚本会拉取上游、打印新提交、自动合并；有冲突时它会告诉你改哪些文件。它不会推送、也不会动游戏目录。

## 方法二：手动操作

```bash
cd C:/Users/Schwarz/projects/Hachimi-Edge
git fetch upstream main
git merge upstream/main        # 冲突就编辑文件 → git add → git commit --no-edit
```

然后构建（见下）。

## 方法三：不装 Rust，用 GitHub 自己构建

仓库里已经带了上游的三个 workflow。网页上：

**Actions → Create Release → Run workflow**

它会在 GitHub 的机器上构建 Windows 和 Android 版，并打包出带安装器的 Release，直接下载安装器就能用，本地什么都不用装。

## 本地构建

需要两样东西：

1. Rust（已装在本机 `~/.cargo`，用前先 `export PATH="$HOME/.cargo/bin:$PATH"`）
2. 上游 CI 用的补丁版 egui（放在 `C:/Users/Schwarz/projects/` 下，一次性准备）：

```bash
cd C:/Users/Schwarz/projects
git clone --filter=blob:none --no-checkout https://github.com/emilk/egui.git egui
cd egui && git checkout 44cdd653e2317d300fb8a6c9c36b03f23991e803 && cd ..
sed -i 's/ui.set_min_width(ui.available_width());/\/\/ ui.set_min_width(ui.available_width());/' \
    egui/crates/egui/src/containers/combo_box.rs
git clone https://github.com/Nekomaru-PKU/egui-directx11.git
cd egui-directx11 && git checkout 903043939a076dc81e122a1c8451755bfb3979c8 && cd ..
```

如果上游哪天换了 egui 的版本，去 `.github/workflows/create_release.yml` 里看它 checkout 的是哪个 commit，把上面两处改掉。

构建：

```bash
cd C:/Users/Schwarz/projects/Hachimi-Edge
export PATH="$HOME/.cargo/bin:$PATH"
export HACHIMI_IGNORE_DIRTY=true          # 必须，否则会因为 Cargo.toml 被改过而拒绝
cp Cargo.toml Cargo.toml.bak && cp Cargo.lock Cargo.lock.bak
cat >> Cargo.toml <<'EOF'
egui-directx11 = { path = '../egui-directx11' }
egui = { path = '../egui/crates/egui' }
egui_extras = { path = '../egui/crates/egui_extras' }
EOF
cargo build --release --target-dir build
mv Cargo.toml.bak Cargo.toml && mv Cargo.lock.bak Cargo.lock
```

产物：`build/release/hachimi.dll`（约 27 MB）。构建完记得把 `Cargo.toml` / `Cargo.lock` 还原，别把临时改动提交上去。

## 装进游戏

游戏目录：`C:\Program Files (x86)\Steam\steamapps\common\UmamusumePrettyDerby_Jpn`

Hachimi 是劫持游戏根目录的 `cri_mana_vpx.dll` 生效的，所以：

```bash
cd "C:/Program Files (x86)/Steam/steamapps/common/UmamusumePrettyDerby_Jpn"
cp cri_mana_vpx.dll cri_mana_vpx.dll.bak      # 备份当前版本
cp C:/Users/Schwarz/projects/Hachimi-Edge/build/release/hachimi.dll cri_mana_vpx.dll
```

启动游戏后按右方向键（默认键位）打开菜单 → 配置编辑器 → 角色替换。

## 上游改动撞车时

你的改动集中在这几个文件，冲突基本只出现在这里：

- `src/core/gui.rs`（角色替换页、下拉）
- `src/core/hachimi.rs`（`replaceGlobalChar` 配置结构）
- `src/il2cpp/sql.rs`（服装查询）
- `src/il2cpp/hook/umamusume/mod.rs`（hook 注册）
- `src/il2cpp/hook/umamusume/CharacterBuildInfo.rs`（新文件，上游不会碰）
- `assets/locales/{en,zh-cn,zh-tw}.yml`（新文案键）

解决冲突的原则：上游的勾子/结构保留上游的，你的角色替换逻辑保留你的；`mod.rs` 里两边都在 `init()` 末尾加调用，合并时两个都留。

## 提醒

- 推送（`git push origin main`）需要 `GH_TOKEN`，可以让我来做，或者你自己在网页上改文件。
- 上游更新后**建议先在自己仓库验证一次**再覆盖游戏里的 dll，因为 hook 依赖的类名可能在游戏更新后变化。

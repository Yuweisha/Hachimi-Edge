# 角色替换（Character Replace）

从 [Trainers' Legend G](https://github.com/MinamiChiwa/Trainers-Legend-G) 移植的全局角色替换功能，
实现在 Hachimi Edge 里，配置入口在游戏内的 **配置编辑器 → 角色替换** 页面。

## 游戏内配置菜单

打开 Hachimi 菜单（默认右方向键）→ 配置编辑器 → 顶部第四个页签「角色替换」。

- **启用角色替换**：总开关。
- **同时替换服装**：对应 TLG 的 `replaceUniversal`。关闭时，原服装 ID 小于 100000 的情况下只替换角色、保留原服装。
- **规则列表**：每条规则 = 原角色 + 新角色 + 服装 + 迷你开关，角色名/服装名都是从 `master.mdb` 读出来的（角色名会跟随翻译数据）。
  - 原角色 / 新角色：带搜索的下拉，直接选角色名，不用记 ID。
  - 服装：下拉列出**新角色**名下的全部服装（带服装名）；新角色没选或没有服装记录时列出全部服装。手工存的旧值也会保留在列表里。
  - 迷你：该规则是否也作用于迷你角色（对应 TLG 的 `replaceMini`）。

## 沿用 TLG 的配置文件

`config.json` 里直接粘贴 TLG 的配置段即可，键名保持原样（`replaceGlobalChar`、`origCharId`、
`newChrId`、`newClothId`、`replaceMini` 都有别名兼容）：

```json
{
    "replaceGlobalChar": {
        "enable": true,
        "data": [
            {
                "origCharId": 1046,
                "newChrId": 1030,
                "newClothId": 103001,
                "replaceMini": false
            }
        ]
    }
}
```

保存后游戏内菜单里也会同步显示同样的规则。

## 实现要点

- Hook `Gallop.CharacterBuildInfo::Rebuild`：角色模型构建时改写 `_charaId` / `_dressId` /
  `_headModelSubId` / `_motionDressId`，并把 `_cardId` 置 -1。
- Hook `Gallop.WorkSingleModeCharaData::GetRaceDressId`：比赛用服装 ID 同样走替换表。
- 服装对应的头部模型 ID 与迷你模型有无，来自 `master.mdb` 的 `dress_data`（`head_sub_id` /
  `have_mini`），首次使用时整表缓存。
- 控制器的生效范围与 TLG 一致（除 Default / HomeTalk / HomeWalk / Mini 之外的场景），
  迷你场景单独处理，原服装没有迷你模型时回退到 `dressId = 2`。

## 构建（Windows）

官方 Windows 构建依赖打了补丁的 egui / egui-directx11，本地构建需要照做：

```bash
# 1. 取补丁版依赖（与上游 CI 同一批 commit）
git clone --filter=blob:none --no-checkout https://github.com/emilk/egui.git egui
cd egui && git checkout 44cdd653e2317d300fb8a6c9c36b03f23991e803 && cd ..
sed -i 's/ui.set_min_width(ui.available_width());/\/\/ ui.set_min_width(ui.available_width());/' \
    egui/crates/egui/src/containers/combo_box.rs
git clone https://github.com/Nekomaru-PKU/egui-directx11.git egui-directx11
cd egui-directx11 && git checkout 903043939a076dc81e122a1c8451755bfb3979c8 && cd ..

# 2. 追加 path 依赖（写在 Cargo.toml 末尾的 [patch.crates-io] 段里）
cat >> Hachimi-Edge/Cargo.toml <<'EOF'
egui-directx11 = { path = '../egui-directx11' }
egui = { path = '../egui/crates/egui' }
egui_extras = { path = '../egui/crates/egui_extras' }
EOF

# 3. 构建
cd Hachimi-Edge
HACHIMI_IGNORE_DIRTY=true cargo build --release --target-dir build
# 产物：build/release/hachimi.dll
```

## 部署

Steam 版赛马娘当前用安装器劫持 `cri_mana_vpx.dll` 加载 Hachimi：

1. 备份游戏目录里的 `cri_mana_vpx.dll`（那是已安装的 Hachimi）。
2. 把新构建的 `hachimi.dll` 复制过去，命名为 `cri_mana_vpx.dll`。
3. 启动游戏，配置编辑器里应能看到「角色替换」页签（`hachimi/` 配置目录与 `config.json` 保持不变）。

## 测试

```bash
cargo test --lib global_char_replace_tests
```

覆盖 TLG 配置段的解析（驼峰键、别名、缺省值）。

# 实验分支约定（exp/*）

所有实验性改动一律提交到 `exp/<主题>` 分支并推送到 origin，**不再留在工作区**。

## 为什么

2026-10-12 的 `motion_dress` 修复只存在于工作区（版本串带 `-dirty`），
10-13 凌晨执行 `git merge origin/main`（Fast-forward）时被覆盖丢失，
导致 1.0 → 1.2.1 全部构建复现"温泉事件卡死/弹窗"。
详见 `exp/motion-dress-fix` 分支的提交说明。

## 规则

1. **实验一开始就建分支**：`git switch -c exp/<短描述>`（基于当前基线 commit）。
2. **每完成一个可用状态就提交**，commit message 写清楚：
   - 改了什么机制
   - 预期效果
   - 已验证 / 未验证
3. **推送**：`git push origin refs/heads/exp/<主题>:refs/heads/exp/<主题>`
4. **构建产物必须归档**（`build_and_archive.sh` → `Hachimi-Edge-builds/`），
   文件名里带 md5 前缀，和分支名对应写入 commit message。
5. **禁止**在 `main` 的工作区里堆未提交的实验改动；`main` 只做发布。
6. 实验转正时：从 `exp/*` 摘取核心改动 → cherry-pick 或手工移植到 `main` →
   删掉探针/日志 → 构建验证 → 再合并。

## 现有分支

| 分支 | 基线 | 内容 |
|---|---|---|
| `exp/motion-dress-fix` | `e2729b5` | `MOTION_DRESS_ID` 保留（通用服装不换动作来源）+ `auto_pick_dress` + `auto_dress` 开关 + TLG 式 `GotoTitleOnError` 拦截（`ErrorSkip.rs`） |
| `exp/diag-cutscene-probe` | `9505e37`（2026-09-30） | 早期诊断实验：`[dialog]`/`[track]`/`[charres]` 日志、`CharacterBuildInfo` 字段 dump、`AssetManager` 资源探针、`auto_dress` 原型 |

## 已知结论（避免重复实验）

- **温泉/剧情事件卡死根因**：`CharacterBuildInfo._motionDressId` 被无条件改写，
  游戏去目标角色身上找原角色的动作轨道 → 装配失败。
- **吞 `GotoTitleOnError` 不能解决问题**：在 Hachimi 框架里挂这个函数
  （`new_hook!` 宏或裸 `MinHook`）都会导致剧情加载卡死，与是否吞掉无关；
  日志证明该函数在卡死路径上**一次都没被调用**。
- **改 RT 尺寸做横屏无效**：显示容器不跟随，画面压瘪。
- **号段保护（1.2.1 的 `SKIP_CUTSCENE_REPLACE`）**只是绕开问题，
  代价是这类事件完全不替换；正确修法是 `motion_dress` 保留。

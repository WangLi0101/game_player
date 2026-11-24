import { useState } from 'react';
import { flushSync } from 'react-dom';
import { createRoot } from 'react-dom/client';
import { iconPath } from './lib/dom.js';
import './tailwind.css';

type ShellActions = {
  preview: boolean;
  onSearch: (value: string) => void;
  onAdd: () => unknown;
  onEnterBigScreen: () => Promise<void>;
};

type ShellView = {
  pageName: string;
  contentLabel: string;
  showIntro: boolean;
  showGameActions: boolean;
  mediaVisible: boolean;
};

const libraryView: ShellView = {
  pageName: '游戏库',
  contentLabel: '游戏库内容',
  showIntro: true,
  showGameActions: true,
  mediaVisible: false,
};

function ShellIcon({ name }: { name: 'search' | 'plus' | 'screen' }) {
  return (
    <svg className="icon" viewBox="0 0 24 24" fill="none" stroke="currentColor"
      strokeWidth="1.7" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"
      dangerouslySetInnerHTML={{ __html: iconPath(name) }} />
  );
}

function HeaderActions({ onSearch, onAdd, onEnterBigScreen, showGameActions }: Omit<ShellActions, 'preview'> & Pick<ShellView, 'showGameActions'>) {
  const [adding, setAdding] = useState(false);
  const [enteringBigScreen, setEnteringBigScreen] = useState(false);

  async function runAction(action: () => unknown, setPending: (pending: boolean) => void) {
    setPending(true);
    try {
      await action();
    } catch {
      // 操作本身负责显示错误提示。
    } finally {
      setPending(false);
    }
  }

  return (
    <div className="topbar-tools">
      <div className="topbar-actions" hidden={!showGameActions}>
        <label className="search-box" id="search-box">
          <ShellIcon name="search" />
          <input id="search" type="search" placeholder="搜索你的游戏" aria-label="搜索游戏"
            onChange={event => onSearch(event.currentTarget.value)} />
        </label>
        <div id="add-action">
          <button type="button" className="primary add-button" aria-label="添加游戏" disabled={adding}
            onClick={() => void runAction(onAdd, setAdding)}>
            <ShellIcon name="plus" />
            <span>添加游戏</span>
          </button>
        </div>
      </div>
      <div id="header-display-action">
        <button type="button" id="big-screen-entry" className="header-icon-button" title="大屏模式"
          aria-label="大屏模式" disabled={enteringBigScreen}
          onClick={() => void runAction(onEnterBigScreen, setEnteringBigScreen)}>
          <ShellIcon name="screen" />
        </button>
      </div>
    </div>
  );
}

function AppShell({ view, actions }: { view: ShellView; actions: ShellActions }) {
  return (
    <>
      <aside className="sidebar" />
      <main className="main">
        <header className="topbar" hidden={view.mediaVisible}>
          <div className="breadcrumb">
            <span>收藏室</span>
            <span className="breadcrumb-slash tw:opacity-45">/</span>
            <strong id="page-name">{view.pageName}</strong>
            {actions.preview && <span className="preview-badge">示例预览</span>}
          </div>
          <HeaderActions {...actions} showGameActions={view.showGameActions} />
        </header>
        <div className="page-intro" id="page-intro" hidden={!view.showIntro}>
          <div>
            <div className="eyebrow">属于你的游戏时光</div>
            <h1>游戏库<span className="title-dot">.</span></h1>
          </div>
          <p>每一次打开，都有新的期待。</p>
        </div>
        <section id="content" aria-label={view.contentLabel} />
        <footer className="page-footer tw:mt-[40px] tw:flex tw:justify-between tw:border-t tw:border-t-(--line) tw:pt-[20px] tw:text-[9px] tw:tracking-[1px] tw:text-(--muted)">
          <span>让热爱，有迹可循。</span>
          <span>游迹</span>
        </footer>
      </main>
      <div id="big-screen-host" />
      <div id="toast" className="toast" role="status" aria-live="polite" />
    </>
  );
}

export function mountAppShell(host: HTMLElement, actions: ShellActions) {
  const root = createRoot(host);
  let view = libraryView;
  const render = () => root.render(<AppShell view={view} actions={actions} />);
  // 现有控制器在启动阶段同步挂载侧栏和内容。
  flushSync(render);

  return (next: Partial<ShellView>) => {
    const nextView = { ...view, ...next };
    if (nextView.pageName === view.pageName && nextView.contentLabel === view.contentLabel &&
      nextView.showIntro === view.showIntro && nextView.showGameActions === view.showGameActions &&
      nextView.mediaVisible === view.mediaVisible) return;
    view = nextView;
    // 标题区高度会影响控制器紧接着恢复的滚动位置。
    flushSync(render);
  };
}

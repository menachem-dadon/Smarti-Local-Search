import {render,screen,fireEvent} from '@testing-library/react';
import {DesignSystemProvider,Menu,Switch,Field,ConfirmDialog} from '../design-system/primitives';
import {he} from '../i18n/he';import {en} from '../i18n/en';
import {highlight} from '../features/search/SearchView';
import {describe,it,expect,vi} from 'vitest';
describe('Accessible shared controls',()=>{
 it('sets RTL direction and renders local Tabler SVG',()=>{const view=render(<DesignSystemProvider theme="dark" dir="rtl"><Field label="חיפוש"/><Menu label="אפשרויות" items={[{id:'open',label:'פתח',icon:'open',onSelect:()=>{}}]}/></DesignSystemProvider>);expect(view.container.firstChild).toHaveAttribute('dir','rtl');expect(view.container.querySelector('svg')).toHaveAttribute('viewBox','0 0 24 24');expect(screen.getByLabelText('חיפוש')).toBeInTheDocument();});
 it('opens menu, invokes real callback and closes on selection',()=>{const action=vi.fn();render(<DesignSystemProvider theme="light"><Menu label="תפריט" items={[{id:'copy',label:'העתק',onSelect:action}]}/></DesignSystemProvider>);fireEvent.click(screen.getByRole('button',{name:'תפריט'}));fireEvent.click(screen.getByRole('menuitem',{name:'העתק'}));expect(action).toHaveBeenCalledOnce();expect(screen.queryByRole('menu')).not.toBeInTheDocument();});
 it('switch exposes state and dispatches user changes',()=>{const action=vi.fn();render(<DesignSystemProvider theme="light"><Switch label="מדיה" checked={false} onCheckedChange={action}/></DesignSystemProvider>);fireEvent.click(screen.getByRole('switch',{name:'מדיה'}));expect(action).toHaveBeenCalledWith(true);});
 it('does not highlight semantic-only passages or execute HTML',()=>{const view=render(<div>{highlight('<script>alert(1)</script> hello','')}</div>);expect(view.container.querySelector('script')).toBeNull();expect(view.container.querySelector('mark')).toBeNull();});
 it('has matching translation keys',()=>{expect(Object.keys(he).sort()).toEqual(Object.keys(en).sort());});
 it('keeps confirmation errors visible, starts on cancel and restores opener focus',()=>{
  const modal=vi.spyOn(HTMLDialogElement.prototype,'showModal').mockImplementation(function(this:HTMLDialogElement){this.open=true});
  const close=vi.spyOn(HTMLDialogElement.prototype,'close').mockImplementation(function(this:HTMLDialogElement){this.open=false});
  const opener=document.createElement('button');document.body.append(opener);opener.focus();
  const view=render(<DesignSystemProvider theme="light" dir="ltr"><ConfirmDialog open title="Clear index" description="Confirm removal" confirmLabel="Clear" error="Index is busy" onConfirm={vi.fn()} onClose={vi.fn()}/></DesignSystemProvider>);
  expect(screen.getByRole('alertdialog')).toContainElement(screen.getByText('Index is busy'));expect(screen.getByRole('button',{name:'Cancel'})).toHaveFocus();
  view.unmount();expect(opener).toHaveFocus();opener.remove();modal.mockRestore();close.mockRestore();
 });
});

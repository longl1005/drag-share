import {describe,it,expect,vi} from 'vitest';
import {useShare} from '../src/useShare';
import type {DesktopBridge,Snapshot} from '../src/types';
function bridge(status:Snapshot['status']='enabled') {
 const snapshot:Snapshot={status,shares:[],address:'smb://10.0.0.2',addressFallback:false,warnings:[]};
 return {snapshot:vi.fn(async()=>snapshot),setEnabled:vi.fn(async()=>({canceled:false})),chooseDirectory:vi.fn(async()=>'/tmp/folder'),add:vi.fn(async()=>({canceled:false})),remove:vi.fn(async()=>({canceled:false})),copy:vi.fn(async()=>{}),reveal:vi.fn(async()=>{}),settings:vi.fn(async()=>{}),hide:vi.fn(async()=>{})} satisfies DesktopBridge;
}
describe('panel controller',()=>{
 it('disables mutations until status is known and enabled',async()=>{const b=bridge('unknown');const c=useShare(b);expect(c.canManage.value).toBe(false);await c.refresh();await c.addDirectory();expect(b.chooseDirectory).not.toHaveBeenCalled();});
 it('serializes fast repeated toggles',async()=>{const b=bridge();let finish!:(x:{canceled:boolean})=>void;b.setEnabled.mockImplementation(()=>new Promise(r=>finish=r));const c=useShare(b);await c.refresh();const p=c.toggle();await c.toggle();expect(b.setEnabled).toHaveBeenCalledTimes(1);finish({canceled:false});await p;expect(c.busy.value).toBe(false);});
 it('canceled elevation is silent and refreshes truth',async()=>{const b=bridge();b.setEnabled.mockResolvedValue({canceled:true});const c=useShare(b);await c.refresh();await c.toggle();expect(c.toast.value).toBeNull();expect(b.snapshot).toHaveBeenCalledTimes(2);});
 it('surfaces errors without turning unknown into disabled',async()=>{const b=bridge();b.snapshot.mockRejectedValue({code:'IO_ERROR',message:'读取失败'});const c=useShare(b);await c.refresh();expect(c.data.value.status).toBe('unknown');expect(c.toast.value?.message).toBe('读取失败');});
 it('ignores stale refresh responses',async()=>{const b=bridge();let resolve!:(v:Snapshot)=>void;b.snapshot.mockImplementationOnce(()=>new Promise(r=>resolve=r));const c=useShare(b);const first=c.refresh();await c.refresh();resolve({status:'disabled',shares:[],address:'old',addressFallback:false,warnings:[]});await first;expect(c.data.value.address).toBe('smb://10.0.0.2');});
 it('rejects directory management when service is disabled',async()=>{const b=bridge('disabled');const c=useShare(b);await c.refresh();await c.drop(['/tmp/a']);expect(b.add).not.toHaveBeenCalled();});
});

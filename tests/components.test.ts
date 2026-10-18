import {mount} from '@vue/test-utils';
import {describe,it,expect} from 'vitest';
import StatusSection from '../src/components/StatusSection.vue';
import ShareList from '../src/components/ShareList.vue';
import DropZone from '../src/components/DropZone.vue';
describe('panel views',()=>{
 it('unknown state is not presented as a disabled service',()=>{const w=mount(StatusSection,{props:{status:'unknown',address:'',fallback:false,busy:false,loading:false}});expect(w.get('h1').text()).toBe('状态待确认');expect(w.get('[role=switch]').attributes('disabled')).toBeDefined();expect(w.text()).toContain('重新读取状态');});
 it('shows actual readonly/guest flags and missing directories',()=>{const w=mount(ShareList,{props:{shares:[{id:'r',name:'共享',path:'/missing',readOnly:true,guestAccess:false,exists:false}],loading:false,canManage:true,unavailable:false}});expect(w.text()).toContain('只读');expect(w.text()).not.toContain('访客');expect(w.find('[aria-label="原目录已不存在"]').exists()).toBe(true);});
 it('read failures do not pretend there are no shared folders',()=>{const w=mount(ShareList,{props:{shares:[],loading:false,canManage:false,unavailable:true}});expect(w.text()).toContain('暂时无法读取共享目录');expect(w.text()).not.toContain('还没有共享目录');});
 it('drop zone disables addition while service is off',()=>{const w=mount(DropZone,{props:{enabled:false,hot:false}});expect(w.get('button').attributes('disabled')).toBeDefined();expect(w.text()).toContain('开启共享后即可添加目录');});
});

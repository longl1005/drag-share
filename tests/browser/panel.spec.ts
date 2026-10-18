import {test,expect} from '@playwright/test';
for(const theme of ['light','dark']){
 for(const state of ['normal','off','empty','loading','error']){
  test(`${theme} ${state}`,async({page})=>{
   const errors:string[]=[];page.on('pageerror',e=>errors.push(e.message));
   await page.goto(`/?preview=${state}&theme=${theme}`);
   if(state==='normal'){await expect(page.getByRole('heading',{name:'共享已开启'})).toBeVisible();await expect(page.getByText('movies',{exact:true})).toBeVisible();}
   if(state==='off'){await expect(page.getByRole('heading',{name:'共享已关闭'})).toBeVisible();await expect(page.getByRole('button',{name:'＋ 添加',exact:true})).toBeDisabled();}
   if(state==='empty'){await expect(page.getByText('还没有共享目录')).toBeVisible();await expect(page.getByRole('button',{name:'选择目录共享',exact:true})).toBeEnabled();}
   if(state==='loading')await expect(page.getByLabel('正在加载共享目录')).toBeVisible();
   if(state==='error'){await expect(page.getByText('暂时无法读取共享目录')).toBeVisible();await expect(page.getByRole('switch')).toBeDisabled();}
   await expect(page.locator('.panel')).toHaveCSS('width','320px');
   expect(await page.evaluate(()=>document.documentElement.scrollHeight)).toBe(440);
   await page.screenshot({animations:'disabled',path:`test-results/${theme}-${state}.png`});expect(errors).toEqual([]);
  });
 }
}
test('toggle, copy feedback and menu',async({page})=>{
 await page.goto('/?preview=normal&theme=light');
 await page.getByRole('button',{name:'关闭共享',exact:true}).click();
 await expect(page.getByRole('heading',{name:'共享已关闭'})).toBeVisible();
 await page.getByRole('button',{name:'开启共享',exact:true}).click();
 await expect(page.getByRole('heading',{name:'共享已开启'})).toBeVisible();
 await page.getByRole('button',{name:'复制',exact:true}).click();await expect(page.getByText('已复制到剪贴板')).toBeVisible();
 await page.getByRole('button',{name:'movies 的操作',exact:true}).click();
 await expect(page.getByRole('menuitem',{name:'复制目录链接'})).toBeVisible();
 await page.getByRole('menuitem',{name:'复制目录链接'}).click();await expect(page.getByRole('menu')).toHaveCount(0);
});

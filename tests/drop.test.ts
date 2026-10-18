import {it,expect} from 'vitest';
import {insideDropZone} from '../src/drop';
it('accepts only current coordinates inside both axes of the actual target',()=>{
 const rect={left:16,top:345,right:304,bottom:413};
 expect(insideDropZone({x:100,y:380},rect,1)).toBe(true);
 expect(insideDropZone({x:200,y:760},rect,2)).toBe(true);
 expect(insideDropZone({x:100,y:430},rect,1)).toBe(false);
 expect(insideDropZone({x:8,y:380},rect,1)).toBe(false);
 expect(insideDropZone({x:100,y:320},rect,1)).toBe(false);
});

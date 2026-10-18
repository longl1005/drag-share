export function insideDropZone(point:{x:number;y:number},rect:{left:number;top:number;right:number;bottom:number},scale:number):boolean {
 const x=point.x/scale,y=point.y/scale;
 return x>=rect.left&&x<=rect.right&&y>=rect.top&&y<=rect.bottom;
}

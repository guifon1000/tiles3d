use bevy::prelude::*; 
use crate::planisphere::{self, Planisphere};
use ndarray::Array2;
use std::{collections::HashMap, io::empty};

#[derive(Clone)]
pub struct Chunk {
    pub size: (usize, usize),
    pub chunk_position: (i32, i32),
    //pub subpixel_origin: (usize, usize, usize),
    pub subpixels: Vec<(usize, usize, usize)>,
    pub refinement_level: usize,
}

impl Chunk {
    pub fn mesh(&self, planisphere: planisphere::Planisphere)-> Mesh//->(Vec<Vec3>, Vec<usize>)
    {
        let mut vec_vertices: Vec<[f32; 3]> = Vec::new();
        let mut vec_indices: Vec<u32> = Vec::new();
        let mut idx: usize = 0;
        eprintln!("size {:?}", self.size);
        for i in 0..(self.size.0 ){

            for j in 0..(self.size.1  ){
                let mut c0 = self.subpixels[0];
                c0 = self.subpixels[i*self.size.1+j];
                let geoc0 = planisphere.subpixel_to_geo(c0.0, c0.1, c0.2);
                let p0 = planisphere.geo_to_gnomonic(geoc0.0, geoc0.1, 0., 0.);
                vec_vertices.push([p0.0 as f32, 1., p0.1 as f32]);
                eprintln!("vertices : {:?} , {} {} ",vec_vertices.len(), i, j);

            }
            let penult =  self.subpixels[i*self.size.0+self.size.1-1];
            let lastj = planisphere.get_neighbour_subpixel(penult.0, penult.1, penult.2, 0, 1);
            let geolastj = planisphere.subpixel_to_geo(lastj.0, lastj.1, lastj.2);
            let plastj = planisphere.geo_to_gnomonic(geolastj.0, geolastj.1, 0., 0.);
            vec_vertices.push([plastj.0 as f32, 1., plastj.1 as f32]);
            eprintln!("vertices : {:?} , {} {} ",vec_vertices.len(), i, self.size.1);
        }
        let penult = self.subpixels[(self.size.0-1)*self.size.1];
        let lasti = planisphere.get_neighbour_subpixel(penult.0, penult.1, penult.2, 1, 0);
        for j in 0..self.size.1{
            let c = (lasti.0, lasti.1, lasti.2 + j );
            let geoc = planisphere.subpixel_to_geo(c.0, c.1, c.2);
            let p = planisphere.geo_to_gnomonic(geoc.0, geoc.1, 0.0, 0.0);
            vec_vertices.push([p.0 as f32, 1., p.1 as f32]);
            eprintln!("vertices : {:?} , {} {} ",vec_vertices.len(), "lasti", j);
        }
        for i in 0..(self.size.0)
        {
            for j in 0..(self.size.1)
            {
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32);
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32 + (self.size.1+2) as u32);
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32 + 1 );
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32);
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32+ (self.size.1 + 1) as u32);
                vec_indices.push(i as u32 * (self.size.1 + 1) as u32 + j as u32+ (self.size.1 + 2) as u32);


            }
        }
        eprintln!("index vector {:?}", vec_indices);
        let mut terrain_mesh = Mesh::new(
        bevy::render::mesh::PrimitiveTopology::TriangleList,
        bevy::render::render_asset::RenderAssetUsages::default()
        );
        terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vec_vertices);
        //terrain_mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        terrain_mesh.insert_indices(bevy::render::mesh::Indices::U32(vec_indices));
        terrain_mesh.compute_smooth_normals();
        terrain_mesh
    }
}



pub struct ChunkPixelStripe {
    pub subpixel_origin: (usize,usize,usize),
    pub chunk_size: (usize, usize),
    //pub active_chunks: HashMap<(i32, i32), Chunk>,
}

impl ChunkPixelStripe {
    pub fn get_chunk_indices(&self, i: usize, j: usize, k:usize, planisphere: Planisphere)->(usize,usize)
    {
           
        let pixel_lon_divisions = planisphere.get_nlon_from_j(j);
        let nw = (k/planisphere.subpixel_divisions)  / self.chunk_size.0;
        let nh = (k%planisphere.subpixel_divisions)  / self.chunk_size.1;
        let chunk_start_subpixel_k = nw* self.chunk_size.0 * planisphere.subpixel_divisions + nh * self.chunk_size.1;

        eprintln!("*************************************************************************");
        eprintln!("Player position : {} {} {} *** subk {} **** nw {} nh {}", i,j,k,planisphere.subpixel_divisions,nw,nh);
        eprintln!("Player position : {} {}  -----  ",(k/planisphere.subpixel_divisions),  (k%planisphere.subpixel_divisions));
        eprintln!("k={}          ********** subpixel_chunk_height= {} subpixel_chunk_width= {} ",k,self.chunk_size.1,self.chunk_size.1);
        eprintln!("chunk starts at {}", chunk_start_subpixel_k);
        eprintln!("pixel {} {} has {} longitudinal subdivisions", i, j, pixel_lon_divisions);
        eprintln!("*************************************************************************");
        (nw , nh )

    }



    
}


#[derive(Resource)]
pub struct ChunksCenter {
    pub subpixel_origin: (usize,usize,usize),
    pub chunk_size: (usize, usize),
    //pub chunk_pixel_stripes: (ChunkPixelStripe, ChunkPixelStripe, ChunkPixelStripe),
}

impl ChunksCenter {
    pub fn new(i: usize, j: usize, k: usize, planisphere: planisphere::Planisphere, chunk_size: (usize, usize)) -> Self {
        eprintln!("new chunks center defined at {} {} {}", i, j, k);

        Self { 
            subpixel_origin: (i, j, k),
            chunk_size: chunk_size,
            //chunk_pixel_stripes: ,
        }
        
    }

    pub fn get_chunk_subpixels(&self, ichunk: i32, jchunk: i32, planisphere: Planisphere)->Vec<(usize, usize, usize)>{
        let mut out = Vec::new();
        let vertical_chunks_per_pixel = planisphere.subpixel_divisions as i32 / self.chunk_size.1 as i32;
        let mut lon_subdivisions = planisphere.get_nlon_from_j(self.subpixel_origin.1);
        let mut origin = self.subpixel_origin;
        eprintln!("chunks_center_origin : {:?}", origin);
        let mut ic = ichunk;
        let mut jc = jchunk;
        if jchunk >= vertical_chunks_per_pixel{
            let jchunk_offset = jchunk / vertical_chunks_per_pixel;
            jc = jc + jchunk_offset;
            origin.1 += jchunk_offset as usize;
            lon_subdivisions = planisphere.get_nlon_from_j(origin.1);
        }


        let mut test = 
            origin.2 as i32 
            + ichunk * self.chunk_size.0 as i32 * planisphere.subpixel_divisions as i32 
            + (jc * self.chunk_size.1 as i32) % lon_subdivisions as i32;
        eprintln!("test ichunk {} jchunk {} {:?}  ",ichunk, jchunk, test);

        let start = test;
        for js in 0..self.chunk_size.1
        {
            let mut j = origin.1;
            for is in 0..self.chunk_size.0
            {
                let mut s = test as usize + js * planisphere.subpixel_divisions + is ;
                let mut i = origin.0;
                if s >= lon_subdivisions * planisphere.subpixel_divisions {
                    let ioffset = s/(lon_subdivisions * planisphere.subpixel_divisions);
                    i += ioffset;
                    s = s % (lon_subdivisions * planisphere.subpixel_divisions);
                }
                eprintln!{"next subpixel : {} {} {}", i, j, s, };
                out.push((i,j,s));
            }
        }
        out
    }

    
}


pub fn get_chunk_indices(
    position: (usize, usize, usize),
    chunks_center: Res<ChunksCenter>,
)
{   
    let k = position.2;
    let (i_origin, j_origin, k_origin) = chunks_center.subpixel_origin;

}
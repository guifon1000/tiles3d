use bevy::prelude::*; 
use crate::planisphere::{self, Planisphere};
use bevy::render::mesh::{Indices, PrimitiveTopology};
use bevy::render::render_asset::RenderAssetUsages;
use ndarray::Array2;
use std::{collections::HashMap, io::empty};

#[derive(Clone)]
pub struct Chunk {
    pub size: (usize, usize),
    pub chunk_position: (i32, i32),
    pub subpixels: Vec<(usize, usize, usize)>,
    pub refinement_level: usize,
}

impl Chunk {
    /// Crée un nouveau chunk avec validation des données
    pub fn new(
        size: (usize, usize),
        chunk_position: (i32, i32),
        subpixels: Vec<(usize, usize, usize)>,
        refinement_level: usize,
    ) -> Result<Self, &'static str> {
        if subpixels.len() != size.0 * size.1 {
            return Err("Le nombre de subpixels ne correspond pas à la taille du chunk");
        }
        
        Ok(Chunk {
            size,
            chunk_position,
            subpixels,
            refinement_level,
        })
    }

    /// Génère le mesh pour ce chunk
    pub fn mesh(&self, planisphere: &planisphere::Planisphere, projection_center: (f64, f64)) -> Mesh {
        let vertices = self.generate_vertices(planisphere, projection_center);
        let indices = self.generate_indices();
        
        self.create_bevy_mesh(vertices, indices)
    }

    /// Génère tous les vertices nécessaires pour le mesh
    fn generate_vertices(
        &self,
        planisphere: &planisphere::Planisphere,
        projection_center: (f64, f64)
    ) -> Vec<[f32; 3]> {
        let (lon_center, lat_center) = projection_center;
        let altitude = 0.2;
        let grid_size = (self.size.0 + 1, self.size.1 + 1);
        let mut vertices = Vec::with_capacity(grid_size.0 * grid_size.1);

        // Génération des vertices en grille (size.0 + 1) x (size.1 + 1)
        for i in 0..grid_size.0 {
            for j in 0..grid_size.1 {
                let subpixel_coords = self.get_vertex_subpixel_coords(i, j, planisphere);
                let geo_coords = planisphere.subpixel_to_geo(
                    subpixel_coords.0, 
                    subpixel_coords.1, 
                    subpixel_coords.2
                );
                let projected = planisphere.geo_to_gnomonic(
                    geo_coords.0, 
                    geo_coords.1, 
                    lon_center, 
                    lat_center
                );
                //test if vertex already in vector
                if vertices.contains(&[projected.0 as f32, altitude, projected.1 as f32]) {
                    eprintln!("WARNING: Duplicate vertex detected at grid ({},{}): subpixel_coords: {:?}, geo_coords: {:?}, projected: {:?}", 
                        i, j, subpixel_coords, geo_coords, projected);
                }
/*                 eprint!("Vertex {} {}: subpixel_coords: {:?}, geo_coords: {:?}, projected: {:?}\n", 
                    i, j, subpixel_coords, geo_coords, projected
                ); */
                let vertex = [projected.0 as f32, altitude, projected.1 as f32];
                
                // Check for invalid coordinates
                if projected.0.is_nan() || projected.1.is_nan() || projected.0.is_infinite() || projected.1.is_infinite() {
                    eprintln!("ERROR: Invalid vertex {},{}: subpixel_coords: {:?}, geo_coords: {:?}, projected: {:?}", 
                        i, j, subpixel_coords, geo_coords, projected);
                }
                
                vertices.push(vertex);
            }
        }
        eprintln!(" vertices generated: {:?}", vertices);
        vertices
    }

/// Détermine les coordonnées subpixel pour un vertex donné    
fn get_vertex_subpixel_coords(
    &self,
    i: usize,
    j: usize,
    planisphere: &planisphere::Planisphere,
) -> (usize, usize, usize) {
    let (rows, cols) = self.size;

    match (i, j) {
        // Interior vertices
        (i, j) if i < cols && j < rows => {
            eprintln!("Getting vertex subpixel coords for interior ({},{})\n", i, j);
            self.subpixels[i * rows + j]
        }
        // Top edge
        (i, j) if i < cols && j == rows => {
            eprintln!("Getting vertex subpixel coords for top edge ({},{})\n", i, j);
            let base_subpixel = self.subpixels[i * rows + (rows - 1)]; // Topmost row of current chunk
            eprintln!(" Base subpixel: {:?}", base_subpixel);
            eprintln!(" Top edge: {:?}", planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                0, 1  
            ));
            planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                0, 1  
            )
        }
        // Right edge 
        (i, j) if i == cols && j < rows => {
            eprintln!("Getting vertex subpixel coords for right edge ({},{})\n", i, j);
            let base_subpixel = self.subpixels[(cols - 1) * rows + j]; // Rightmost column of current chunk
            eprintln!(" Base subpixel: {:?}", base_subpixel);
            eprintln!(" Right edge: {:?}", planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                1, 0  
            ));
            planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                1, 0  
            )
        }

                (i, j) if i == cols && j == rows => {
            eprintln!("Getting vertex subpixel coords for nortWest corner ({},{})\n", i, j);
            let base_subpixel = self.subpixels[rows  * cols -1]; // Rightmost column of current chunk
            eprintln!(" Base subpixel: {:?}", base_subpixel);
            eprintln!(" NorthWest edge: {:?}", planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                1, 1  
            ));
            planisphere.get_neighbour_subpixel(
                base_subpixel.0,
                base_subpixel.1,
                base_subpixel.2,
                1, 1  
            )
        }
        


        _ => unreachable!("Invalid vertex coordinates ({},{})", i, j),
    }
}
    /// Détermine les coordonnées subpixel pour un vertex donné
    fn get_vertex_subpixel_coords0000(
        &self,
        i: usize,
        j: usize,
        planisphere: &planisphere::Planisphere
    ) -> (usize, usize, usize) {
        let (rows, cols) = self.size;

        match (i, j) {
            // Vertices intérieurs du chunk
            (i, j) if i < cols && j < rows => {
            eprint!("Getting vertex subpixel coords for grid A ({},{})\n", i, j);
                self.subpixels[i * cols + j]
            }
            // Bord sup
            (i, j) if i < cols && j == rows => {
            eprint!("Getting vertex subpixel coords for grid B ({},{})", i, j);
                let base_subpixel = self.subpixels[i * cols + (cols - 1)];
                planisphere.get_neighbour_subpixel(
                    base_subpixel.0, 
                    base_subpixel.1, 
                    base_subpixel.2, 
                    0, 1
                )
            }
            // Bord est
            (i, j) if i == cols && j < rows => {
            eprint!("Getting vertex subpixel coords for grid C ({},{})", i, j);
                let base_subpixel = self.subpixels[(rows - 1) * cols + j];
                planisphere.get_neighbour_subpixel(
                    base_subpixel.0, 
                    base_subpixel.1, 
                    base_subpixel.2, 
                    1, 0
                )
            }

            (i, j) if i == cols && j == rows => {
            eprint!("Getting vertex subpixel coords for grid D ({},{})", i, j);
                let base_subpixel = self.subpixels[rows  * cols  -1];
                planisphere.get_neighbour_subpixel(
                    base_subpixel.0, 
                    base_subpixel.1, 
                    base_subpixel.2, 
                    1, 1
                )
            }


            _ => unreachable!("Coordonnées de vertex invalides")
        }
    }

    /// Génère les indices pour les triangles du mesh
    fn generate_indices(&self) -> Vec<u32> {
        let (rows, cols) = self.size;
        let mut indices = Vec::with_capacity(rows * cols * 6); // 2 triangles par quad, 3 indices par triangle
        let max_vertex = (rows + 1) * (cols + 1) - 1;

        for i in 0..rows {
            for j in 0..cols {
                let base_idx = (i * (cols + 1) + j) as u32;
                let stride = (cols + 1) as u32;
                
                // Validate indices are within bounds
                let idx1 = base_idx;
                let idx2 = base_idx + stride;
                let idx3 = base_idx + 1;
                let idx4 = base_idx + stride + 1;
                
                if idx4 > max_vertex as u32 {
                    eprintln!("ERROR: Index out of bounds! max_vertex={}, trying to use={}", max_vertex, idx4);
                    continue;
                }
                
                // eprintln!("Quad ({},{}) uses vertices: {} {} {} {}", i, j, idx1, idx2, idx3, idx4);
                
                // Premier triangle du quad
                indices.extend_from_slice(&[idx1, idx3, idx2]);
                // Deuxième triangle du quad  
                indices.extend_from_slice(&[idx2, idx3, idx4]);
            }
        }

        eprintln!("Generated {} indices for {}x{} grid", indices.len(), rows, cols);
        eprintln!(" indices generated: {:?}", indices);
        indices
    }

    /// Crée le mesh Bevy final
    fn create_bevy_mesh(&self, vertices: Vec<[f32; 3]>, indices: Vec<u32>) -> Mesh {
        eprintln!("Creating mesh with {} vertices and {} indices ({} triangles)", 
            vertices.len(), indices.len(), indices.len() / 3);
        
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::RENDER_WORLD | RenderAssetUsages::MAIN_WORLD,
        );

        // Attributs de base
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, vertices);
        mesh.insert_indices(Indices::U32(indices));

        // Génération des normales et coordonnées UV
        mesh.compute_smooth_normals();
        
        // Optionnel : générer des coordonnées UV basiques
        if let Some(uvs) = self.generate_uvs() {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        }

        // Debug mesh attributes
        eprintln!("Mesh attributes: {:?}", mesh.attributes().map(|(id, _)| id).collect::<Vec<_>>());
        eprintln!("Mesh has indices: {}", mesh.indices().is_some());
        eprintln!("Mesh primitive topology: {:?}", mesh.primitive_topology());

        mesh
    }

    /// Génère des coordonnées UV basiques pour le texture mapping
    fn generate_uvs(&self) -> Option<Vec<[f32; 2]>> {
        let (rows, cols) = self.size;
        let grid_size = (rows + 1, cols + 1);
        let mut uvs = Vec::with_capacity(grid_size.0 * grid_size.1);

        for i in 0..grid_size.0 {
            for j in 0..grid_size.1 {
                let u = j as f32 / cols as f32;
                let v = i as f32 / rows as f32;
                uvs.push([u, v]);
            }
        }

        Some(uvs)
    }

    /// Méthode utilitaire pour déboguer
    pub fn validate_mesh_topology(&self) -> Result<(), String> {
        let vertex_count = (self.size.0 + 1) * (self.size.1 + 1);
        let expected_triangle_count = self.size.0 * self.size.1 * 2;
        let expected_index_count = expected_triangle_count * 3;

        println!("Chunk validation:");
        println!("  Taille: {:?}", self.size);
        println!("  Vertices attendus: {}", vertex_count);
        println!("  Triangles attendus: {}", expected_triangle_count);
        println!("  Indices attendus: {}", expected_index_count);

        Ok(())
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

    /// Get the starting subpixel coordinate (i,j,k) for a specific chunk
    pub fn get_chunk_start_subpixel(&self, ichunk: i32, jchunk: i32, planisphere: Planisphere) -> (usize, usize, usize) {
        let origin_i = self.subpixel_origin.0;
        let origin_j = self.subpixel_origin.1;
        let origin_k = self.subpixel_origin.2;
        let mut i_start = origin_i;
        let mut j_start = origin_j;
        let mut k_start = origin_k as i32 + (ichunk * self.chunk_size.0 as i32 * planisphere.subpixel_divisions as i32) + (jchunk * self.chunk_size.1 as i32);
        
        // Handle negative k values properly
        let nlon = planisphere.get_nlon_from_j(j_start);
        let max_k = (nlon * planisphere.subpixel_divisions) as i32;
        while k_start < 0 {
            k_start += max_k;
        }
        eprintln!(" DBG : chunk ({},{}) initial start at ({} {} {}) nlon={} subpixel_divisions={}", ichunk, jchunk, i_start, j_start, k_start, nlon, planisphere.subpixel_divisions);
        let vertical_chunks_per_pixel = planisphere.subpixel_divisions / self.chunk_size.1;
        if (origin_k as i32 + (jchunk * self.chunk_size.1 as i32)) >= planisphere.subpixel_divisions as i32 {
            j_start += ((origin_k as i32 + (jchunk * self.chunk_size.1 as i32)) / planisphere.subpixel_divisions as i32) as usize;
            k_start = (origin_k as i32 + (jchunk * self.chunk_size.1 as i32)) % planisphere.subpixel_divisions as i32 + (ichunk * self.chunk_size.0 as i32 * planisphere.subpixel_divisions as i32);
            eprintln!(" correction of j_start overflow, new i_start={}, new j_start ={} k_start={}", i_start, j_start, k_start);
            
            // Handle negative k values after recalculation
            let nlon_corrected = planisphere.get_nlon_from_j(j_start);
            let max_k_corrected = (nlon_corrected * planisphere.subpixel_divisions) as i32;
            while k_start < 0 {
                k_start += max_k_corrected;
            }
        }

        if k_start >= (nlon * planisphere.subpixel_divisions) as i32 {
            i_start += (k_start / (nlon * planisphere.subpixel_divisions) as i32) as usize;
            k_start = k_start % (nlon * planisphere.subpixel_divisions) as i32;
            eprintln!(" correction of k_start overflow, new i_start={}, new j_start ={} k_start={}", i_start, j_start, k_start);

        }
        
        
        eprintln!(" DBG : chunk ({},{}) starts at ({} {} {})", ichunk, jchunk, i_start, j_start, k_start);

        
        (i_start as usize, j_start, k_start as usize)
    }

    pub fn get_chunk_subpixels(&self, ichunk: i32, jchunk: i32, planisphere: Planisphere) -> Vec<(usize, usize, usize)> {
        let mut subpixel_list = Vec::new();
        
        // Get the starting subpixel for this chunk
        let (start_i, start_j, start_k) = self.get_chunk_start_subpixel(ichunk, jchunk, planisphere.clone());
        
        // Generate all subpixels for this chunk by adding offsets from the start
        for col in 0..self.chunk_size.0 {
            for row in 0..self.chunk_size.1 {
                let subpixel_i = start_i ;
                let subpixel_j = start_j ;
                let subpixel_k = start_k + col * planisphere.subpixel_divisions + row;
                
                subpixel_list.push((subpixel_i, subpixel_j, subpixel_k));
            }
        }
        
        eprintln!("generated {} subpixels for chunk ({},{})", subpixel_list.len(), ichunk, jchunk);
        subpixel_list
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
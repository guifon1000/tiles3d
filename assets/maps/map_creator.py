import numpy as np
from PIL import Image
import math

class PerlinNoise:
    def __init__(self, seed=0):
        np.random.seed(seed)
        # Generate permutation table
        self.p = np.arange(256)
        np.random.shuffle(self.p)
        self.p = np.tile(self.p, 2)  # Duplicate for easier indexing
        
        # Generate gradient vectors
        self.gradients = []
        for i in range(256):
            angle = np.random.uniform(0, 2 * np.pi)
            self.gradients.append([np.cos(angle), np.sin(angle)])
        self.gradients = np.array(self.gradients)
    
    def fade(self, t):
        """Fade function for smooth interpolation"""
        return t * t * t * (t * (t * 6 - 15) + 10)
    
    def lerp(self, t, a, b):
        """Linear interpolation"""
        return a + t * (b - a)
    
    def grad(self, hash_val, x, y):
        """Gradient function"""
        grad_vec = self.gradients[hash_val & 255]
        return grad_vec[0] * x + grad_vec[1] * y
    
    def noise2d(self, x, y, period_x=None, period_y=None):
        """Generate 2D Perlin noise with optional periodicity"""
        if period_x is not None:
            x = x % period_x
        if period_y is not None:
            y = y % period_y
            
        # Grid coordinates
        xi = int(x) & 255
        yi = int(y) & 255
        
        # Fractional part
        xf = x - int(x)
        yf = y - int(y)
        
        # Fade curves
        u = self.fade(xf)
        v = self.fade(yf)
        
        # Hash coordinates
        aa = self.p[self.p[xi] + yi]
        ab = self.p[self.p[xi] + yi + 1]
        ba = self.p[self.p[xi + 1] + yi]
        bb = self.p[self.p[xi + 1] + yi + 1]
        
        # Gradient calculations
        g1 = self.grad(aa, xf, yf)
        g2 = self.grad(ba, xf - 1, yf)
        g3 = self.grad(ab, xf, yf - 1)
        g4 = self.grad(bb, xf - 1, yf - 1)
        
        # Interpolate
        x1 = self.lerp(u, g1, g2)
        x2 = self.lerp(u, g3, g4)
        
        return self.lerp(v, x1, x2)

def generate_sphere_texture(seed, nx, ny, output_filename="sphere_texture.png"):
    """
    Generate a texture with 4 different Perlin noises for RGBA channels
    with proper periodicity for sphere mapping
    """
    # Create noise generator
    noise = PerlinNoise(seed)
    
    # Initialize RGBA arrays
    rgba = np.zeros((ny, nx, 4), dtype=np.uint8)
    
    # Parameters for different noise layers
    noise_params = [
        {"scale": 24.0, "octaves": 4, "persistence": 0.5},   # Red channel
        {"scale": 12.0, "octaves": 3, "persistence": 0.6},  # Green channel
        {"scale": 6.0, "octaves": 5, "persistence": 0.4},   # Blue channel
        {"scale": 4.0, "octaves": 3, "persistence": 0.7}   # Alpha channel
    ]
    
    # Generate noise for each channel
    for channel in range(4):
        params = noise_params[channel]
        channel_data = np.zeros((ny, nx))
        
        # Generate multiple octaves
        for octave in range(params["octaves"]):
            frequency = params["scale"] * (2 ** octave)
            amplitude = params["persistence"] ** octave
            
            # For sphere mapping, we need to ensure periodicity
            # The texture wraps around horizontally (longitude) but not vertically (latitude)
            for y in range(ny):
                for x in range(nx):
                    # Map to sphere coordinates
                    # x corresponds to longitude (0 to 2π) - needs to be periodic
                    # y corresponds to latitude (0 to π) - no periodicity needed
                    
                    # Normalize coordinates
                    norm_x = x / nx
                    norm_y = y / ny
                    
                    # Scale by frequency
                    sample_x = norm_x * frequency
                    sample_y = norm_y * frequency
                    
                    # Generate noise with horizontal periodicity
                    noise_val = noise.noise2d(
                        sample_x, 
                        sample_y, 
                        period_x=frequency,  # Horizontal periodicity
                        period_y=None        # No vertical periodicity
                    )
                    
                    channel_data[y, x] += noise_val * amplitude
        
        # Normalize to 0-255 range
        min_val = np.min(channel_data)
        max_val = np.max(channel_data)
        
        if max_val > min_val:
            channel_data = (channel_data - min_val) / (max_val - min_val)
        else:
            channel_data = np.zeros_like(channel_data)
        
        # Apply different transformations for each channel
        if channel == 0:  # Red - high contrast
            channel_data = np.power(channel_data, 0.7)
        elif channel == 1:  # Green - smoother
            channel_data = np.sin(channel_data * np.pi) ** 2
        elif channel == 2:  # Blue - inverse
            channel_data = 1.0 - channel_data
        else:  # Alpha - keep as is but ensure it's not too transparent
            channel_data = 0.3 + 0.7 * channel_data
        
        # Convert to 0-255 range
        rgba[:, :, channel] = (channel_data * 255).astype(np.uint8)
    
    # Create and save image
    image = Image.fromarray(rgba, 'RGBA')
    image.save(output_filename)
    
    print(f"Generated {nx}x{ny} sphere texture: {output_filename}")
    print(f"Used seed: {seed}")
    
    return rgba

def main():
    # Parameters
    seed = 42
    nx = 700  # Width (longitude)
    ny = 350  # Height (latitude)
    
    # Generate the texture
    texture = generate_sphere_texture(seed, nx, ny, "sphere_texture.png")
    
    # Optional: Generate a preview showing each channel separately
    fig_data = []
    
    # Create individual channel previews
    for i, channel_name in enumerate(['Red', 'Green', 'Blue', 'Alpha']):
        channel_img = np.zeros((ny, nx, 4), dtype=np.uint8)
        channel_img[:, :, i] = texture[:, :, i]
        channel_img[:, :, 3] = 255  # Full alpha for preview
        
        preview_img = Image.fromarray(channel_img, 'RGBA')
        preview_img.save(f"channel_{channel_name.lower()}_preview.png")
        print(f"Generated {channel_name} channel preview")

if __name__ == "__main__":
    main()

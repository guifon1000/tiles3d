import os
import sys
from PIL import Image, ImageTk, ImageEnhance
import glob
import tkinter as tk
from tkinter import ttk, messagebox
import threading

def enhance_contrast_and_saturation(image, contrast_factor=1.3, saturation_factor=1.4, brightness_factor=1.1):
    """Enhance contrast, saturation, and brightness to make colors more vibrant"""
    # Enhance contrast
    contrast_enhancer = ImageEnhance.Contrast(image)
    image = contrast_enhancer.enhance(contrast_factor)
    
    # Enhance color saturation
    color_enhancer = ImageEnhance.Color(image)
    image = color_enhancer.enhance(saturation_factor)
    
    # Slightly increase brightness for more vibrant appearance
    brightness_enhancer = ImageEnhance.Brightness(image)
    image = brightness_enhancer.enhance(brightness_factor)
    
    return image

def convert_to_16bit_colors(image):
    """Convert image to 16-bit color format (RGB565-like) with enhanced vibrancy and contrast"""
    # Convert to RGB if not already
    if image.mode != 'RGB':
        image = image.convert('RGB')
    
    # Enhance contrast and saturation first
    image = enhance_contrast_and_saturation(image)
    
    # Reduce color depth to simulate 16-bit colors
    # 5 bits for red, 6 bits for green, 5 bits for blue
    pixels = image.load()
    width, height = image.size
    
    for y in range(height):
        for x in range(width):
            r, g, b = pixels[x, y]
            
            # Apply gamma correction for better color representation
            r = min(255, int(pow(r / 255.0, 0.8) * 255))
            g = min(255, int(pow(g / 255.0, 0.8) * 255))
            b = min(255, int(pow(b / 255.0, 0.8) * 255))
            
            # Reduce to 5-bit red (0-31), 6-bit green (0-63), 5-bit blue (0-31)
            r = (r >> 3) << 3  # Keep top 5 bits
            g = (g >> 2) << 2  # Keep top 6 bits
            b = (b >> 3) << 3  # Keep top 5 bits
            
            pixels[x, y] = (r, g, b)
    
    return image

def process_image(img_path):
    """Process a single image: resize to 14x14, convert to 16-bit colors, add grey border"""
    try:
        # Open and resize to 14x14
        img = Image.open(img_path)
        img = img.resize((14, 14), Image.Resampling.LANCZOS)
        
        # Convert to 16-bit color format
        img = convert_to_16bit_colors(img)
        
        # Create 16x16 image with grey border
        bordered_img = Image.new('RGB', (16, 16), (128, 128, 128))  # Grey background
        bordered_img.paste(img, (1, 1))  # Paste 14x14 image with 1px offset
        
        return bordered_img
    except Exception as e:
        print(f"Error processing {img_path}: {e}")
        return None

def create_transparent_texture():
    """Create a transparent 14x14 texture with grey border"""
    # Create transparent 14x14 texture
    transparent_img = Image.new('RGBA', (14, 14), (0, 0, 0, 0))
    
    # Create 16x16 with grey border
    bordered_img = Image.new('RGB', (16, 16), (128, 128, 128))
    
    # Convert transparent image to RGB for pasting
    rgb_transparent = Image.new('RGB', (14, 14), (255, 255, 255))
    rgb_transparent.putalpha(0)  # Make it as transparent as possible in RGB
    
    # Since we're working with RGB, we'll use a light grey for "transparent" areas
    transparent_rgb = Image.new('RGB', (14, 14), (200, 200, 200))
    bordered_img.paste(transparent_rgb, (1, 1))
    
    return bordered_img

def create_texture_atlas(ordered_image_files=None):
    """Main function to create the texture atlas"""
    # Get all image files from img/ folder
    img_folder = "img"
    if not os.path.exists(img_folder):
        print(f"Error: {img_folder} folder not found!")
        return
    
    if ordered_image_files is None:
        # Supported image formats
        extensions = ['*.png', '*.jpg', '*.jpeg', '*.bmp', '*.gif', '*.tiff']
        image_files = []
        
        for ext in extensions:
            image_files.extend(glob.glob(os.path.join(img_folder, ext)))
            image_files.extend(glob.glob(os.path.join(img_folder, ext.upper())))
    else:
        image_files = ordered_image_files
    
    print(f"Found {len(image_files)} image files")
    
    # Process images
    processed_textures = []
    
    for img_path in image_files:
        print(f"Processing: {os.path.basename(img_path)}")
        texture = process_image(img_path)
        if texture:
            processed_textures.append(texture)
    
    # Create 16x16 texture atlas (256 total textures)
    atlas_size = 16 * 16  # 256 textures
    texture_size = 16  # Each texture is 16x16
    atlas_width = atlas_height = 16 * texture_size  # 256x256 pixels
    
    # Create the atlas image
    atlas = Image.new('RGB', (atlas_width, atlas_height), (0, 0, 0))
    
    # Fill the atlas
    for i in range(atlas_size):
        row = i // 16
        col = i % 16
        x = col * texture_size
        y = row * texture_size
        
        if i < len(processed_textures):
            # Use processed texture
            atlas.paste(processed_textures[i], (x, y))
        else:
            # Use transparent texture for empty spots
            transparent_texture = create_transparent_texture()
            atlas.paste(transparent_texture, (x, y))
    
    # Save the texture atlas
    atlas.save('texture_atlas.png')
    print(f"Texture atlas created: texture_atlas.png")
    print(f"Atlas size: {atlas_width}x{atlas_height} pixels")
    print(f"Used {len(processed_textures)} textures, filled {atlas_size - len(processed_textures)} empty spots")

class TextureAtlasUI:
    def __init__(self, master):
        self.master = master
        self.master.title("Texture Atlas Creator")
        self.master.geometry("1200x800")
        
        # Load available textures
        self.load_textures()
        
        # Create UI elements
        self.create_widgets()
        
        # Selected texture order and available textures tracking
        self.selected_textures = []
        self.available_textures = self.image_files.copy()
        
        # Update available list and initialize UI
        self.selected_available_texture = None
        self.update_available_list()
        self.update_selected_list()
        self.update_atlas_preview()
        
    def load_textures(self):
        """Load all texture files from img folder"""
        img_folder = "img"
        if not os.path.exists(img_folder):
            messagebox.showerror("Error", f"{img_folder} folder not found!")
            return
        
        # Supported image formats
        extensions = ['*.png', '*.jpg', '*.jpeg', '*.bmp', '*.gif', '*.tiff']
        self.image_files = []
        
        for ext in extensions:
            self.image_files.extend(glob.glob(os.path.join(img_folder, ext)))
            self.image_files.extend(glob.glob(os.path.join(img_folder, ext.upper())))
        
        # Sort files alphabetically
        self.image_files.sort()
        
        # Create thumbnail images for UI display
        self.thumbnails = {}
        self.processed_thumbnails = {}
        for img_path in self.image_files:
            try:
                # Original thumbnail for display
                img = Image.open(img_path)
                img.thumbnail((48, 48), Image.Resampling.LANCZOS)
                self.thumbnails[img_path] = ImageTk.PhotoImage(img)
                
                # Processed thumbnail (what will appear in atlas) - smaller for preview
                processed_img = process_image(img_path)
                if processed_img:
                    processed_img = processed_img.resize((16, 16), Image.Resampling.LANCZOS)
                    self.processed_thumbnails[img_path] = ImageTk.PhotoImage(processed_img)
            except Exception as e:
                print(f"Error creating thumbnail for {img_path}: {e}")
    
    def create_widgets(self):
        """Create the UI widgets"""
        # Main frame
        main_frame = ttk.Frame(self.master, padding="10")
        main_frame.grid(row=0, column=0, sticky=(tk.W, tk.E, tk.N, tk.S))
        
        # Configure grid weights
        self.master.columnconfigure(0, weight=1)
        self.master.rowconfigure(0, weight=1)
        main_frame.columnconfigure(0, weight=1)
        main_frame.columnconfigure(2, weight=1)
        main_frame.rowconfigure(1, weight=1)
        
        # Title
        title_label = ttk.Label(main_frame, text="Texture Atlas Creator", font=("Arial", 16, "bold"))
        title_label.grid(row=0, column=0, columnspan=4, pady=(0, 10))
        
        # Left panel - Available textures
        available_frame = ttk.LabelFrame(main_frame, text="Available Textures", padding="5")
        available_frame.grid(row=1, column=0, sticky=(tk.W, tk.E, tk.N, tk.S), padx=(0, 5))
        
        # Create canvas for available textures with scrollbar
        available_canvas = tk.Canvas(available_frame, width=200, height=400)
        available_scrollbar = ttk.Scrollbar(available_frame, orient="vertical", command=available_canvas.yview)
        self.available_scrollable_frame = ttk.Frame(available_canvas)
        
        self.available_scrollable_frame.bind(
            "<Configure>",
            lambda e: available_canvas.configure(scrollregion=available_canvas.bbox("all"))
        )
        
        available_canvas.create_window((0, 0), window=self.available_scrollable_frame, anchor="nw")
        available_canvas.configure(yscrollcommand=available_scrollbar.set)
        
        available_canvas.pack(side="left", fill="both", expand=True)
        available_scrollbar.pack(side="right", fill="y")
        
        # Control buttons frame
        control_frame = ttk.Frame(main_frame)
        control_frame.grid(row=1, column=1, padx=5, sticky=tk.N)
        
        ttk.Button(control_frame, text="Add →", command=self.add_texture).pack(pady=2, fill=tk.X)
        ttk.Button(control_frame, text="← Remove", command=self.remove_texture).pack(pady=2, fill=tk.X)
        ttk.Button(control_frame, text="↑ Move Up", command=self.move_up).pack(pady=2, fill=tk.X)
        ttk.Button(control_frame, text="↓ Move Down", command=self.move_down).pack(pady=2, fill=tk.X)
        ttk.Button(control_frame, text="Clear All", command=self.clear_all).pack(pady=10, fill=tk.X)
        
        # Middle panel - Selected textures
        selected_frame = ttk.LabelFrame(main_frame, text="Selected Textures (Atlas Order)", padding="5")
        selected_frame.grid(row=1, column=2, sticky=(tk.W, tk.E, tk.N, tk.S), padx=5)
        
        # Selected textures listbox with scrollbar and thumbnails
        selected_scroll = ttk.Scrollbar(selected_frame)
        selected_scroll.pack(side=tk.RIGHT, fill=tk.Y)
        
        self.selected_listbox = tk.Listbox(selected_frame, yscrollcommand=selected_scroll.set, height=20)
        self.selected_listbox.pack(side=tk.LEFT, fill=tk.BOTH, expand=True)
        selected_scroll.config(command=self.selected_listbox.yview)
        
        # Right panel - Atlas preview
        preview_frame = ttk.LabelFrame(main_frame, text="Atlas Preview", padding="5")
        preview_frame.grid(row=1, column=3, sticky=(tk.W, tk.E, tk.N, tk.S), padx=(5, 0))
        
        # Canvas for atlas preview
        self.atlas_canvas = tk.Canvas(preview_frame, width=320, height=320, bg="black")
        self.atlas_canvas.pack(padx=5, pady=5)
        
        # Preview info
        self.preview_info = ttk.Label(preview_frame, text="Atlas: 16x16 grid (256x256px)")
        self.preview_info.pack(pady=5)
        
        # Bottom frame for create button
        bottom_frame = ttk.Frame(main_frame)
        bottom_frame.grid(row=2, column=0, columnspan=4, pady=(10, 0), sticky=(tk.W, tk.E))
        
        # Status label
        self.status_label = ttk.Label(bottom_frame, text=f"Found {len(self.image_files)} textures")
        self.status_label.pack(side=tk.LEFT)
        
        # Create atlas button
        create_button = ttk.Button(bottom_frame, text="Create Atlas", command=self.create_atlas)
        create_button.pack(side=tk.RIGHT)
        
    def update_available_list(self):
        """Update the available textures display with thumbnails"""
        # Clear existing widgets
        for widget in self.available_scrollable_frame.winfo_children():
            widget.destroy()
        
        # Add texture buttons with previews
        self.available_buttons = {}
        for img_path in self.available_textures:
            frame = ttk.Frame(self.available_scrollable_frame)
            frame.pack(fill=tk.X, padx=2, pady=1)
            
            # Texture preview
            if img_path in self.thumbnails:
                preview_label = ttk.Label(frame, image=self.thumbnails[img_path])
                preview_label.pack(side=tk.LEFT, padx=(0, 5))
            
            # Texture name button
            name = os.path.basename(img_path)
            btn = ttk.Button(frame, text=name, command=lambda p=img_path: self.select_available_texture(p))
            btn.pack(side=tk.LEFT, fill=tk.X, expand=True)
            
            self.available_buttons[img_path] = (frame, btn)
    
    def select_available_texture(self, img_path):
        """Select a texture from available list"""
        self.selected_available_texture = img_path
    
    def update_atlas_preview(self):
        """Update the atlas preview canvas"""
        self.atlas_canvas.delete("all")
        
        # Draw grid
        grid_size = 16
        cell_size = 20  # 320/16 = 20 pixels per cell
        
        # Draw grid lines
        for i in range(grid_size + 1):
            x = i * cell_size
            y = i * cell_size
            self.atlas_canvas.create_line(x, 0, x, 320, fill="gray", width=1)
            self.atlas_canvas.create_line(0, y, 320, y, fill="gray", width=1)
        
        # Draw selected textures
        for i, img_path in enumerate(self.selected_textures):
            if i >= 256:  # Max 256 textures
                break
            
            row = i // 16
            col = i % 16
            x = col * cell_size + 2
            y = row * cell_size + 2
            
            # Draw thumbnail if available
            if img_path in self.processed_thumbnails:
                try:
                    # Use processed thumbnail
                    self.atlas_canvas.create_image(x + 10, y + 10, anchor=tk.CENTER, 
                                                 image=self.processed_thumbnails[img_path])
                except:
                    # Fallback to colored rectangle
                    self.atlas_canvas.create_rectangle(x, y, x + 18, y + 18, 
                                                     fill="blue", outline="white")
            else:
                # Empty slot
                self.atlas_canvas.create_rectangle(x, y, x + 18, y + 18, 
                                                 fill="gray", outline="white")
    
    def add_texture(self):
        """Add selected texture to atlas order"""
        if hasattr(self, 'selected_available_texture'):
            texture_path = self.selected_available_texture
            
            # Add to selected list if not already there
            if texture_path not in self.selected_textures and texture_path in self.available_textures:
                self.selected_textures.append(texture_path)
                self.available_textures.remove(texture_path)
                
                # Update displays
                self.update_available_list()
                self.update_selected_list()
                self.update_atlas_preview()
                self.update_status()
    
    def remove_texture(self):
        """Remove selected texture from atlas order"""
        selection = self.selected_listbox.curselection()
        if selection:
            index = selection[0]
            texture_path = self.selected_textures.pop(index)
            self.available_textures.append(texture_path)
            self.available_textures.sort()  # Keep alphabetical order
            
            # Update displays
            self.update_available_list()
            self.update_selected_list()
            self.update_atlas_preview()
            self.update_status()
    
    def move_up(self):
        """Move selected texture up in order"""
        selection = self.selected_listbox.curselection()
        if selection and selection[0] > 0:
            index = selection[0]
            # Swap in list
            self.selected_textures[index], self.selected_textures[index-1] = \
                self.selected_textures[index-1], self.selected_textures[index]
            
            # Update displays
            self.update_selected_list()
            self.update_atlas_preview()
            
            # Maintain selection
            self.selected_listbox.select_set(index-1)
    
    def move_down(self):
        """Move selected texture down in order"""
        selection = self.selected_listbox.curselection()
        if selection and selection[0] < len(self.selected_textures) - 1:
            index = selection[0]
            # Swap in list
            self.selected_textures[index], self.selected_textures[index+1] = \
                self.selected_textures[index+1], self.selected_textures[index]
            
            # Update displays
            self.update_selected_list()
            self.update_atlas_preview()
            
            # Maintain selection
            self.selected_listbox.select_set(index+1)
    
    def clear_all(self):
        """Clear all selected textures"""
        # Move all selected textures back to available
        self.available_textures.extend(self.selected_textures)
        self.available_textures.sort()
        self.selected_textures.clear()
        
        # Update displays
        self.update_available_list()
        self.update_selected_list()
        self.update_atlas_preview()
        self.update_status()
    
    def update_selected_list(self):
        """Update the selected textures listbox"""
        self.selected_listbox.delete(0, tk.END)
        for img_path in self.selected_textures:
            self.selected_listbox.insert(tk.END, f"{len(self.selected_textures) - self.selected_textures.index(img_path)}: {os.path.basename(img_path)}")
        
        # Show index numbers for atlas positions
        self.selected_listbox.delete(0, tk.END)
        for i, img_path in enumerate(self.selected_textures):
            self.selected_listbox.insert(tk.END, f"{i:3d}: {os.path.basename(img_path)}")
    
    def update_status(self):
        """Update status label"""
        total = len(self.image_files)
        selected = len(self.selected_textures)
        self.status_label.config(text=f"Found {total} textures, {selected} selected")
    
    def create_atlas(self):
        """Create the texture atlas with selected order"""
        if not self.selected_textures:
            messagebox.showwarning("Warning", "No textures selected! Using all textures in default order.")
            textures_to_use = None
        else:
            textures_to_use = self.selected_textures
        
        # Disable create button during processing
        for widget in self.master.winfo_children():
            self.disable_widget(widget)
        
        self.status_label.config(text="Creating atlas...")
        self.master.update()
        
        # Run atlas creation in separate thread to prevent UI freezing
        def create_thread():
            try:
                create_texture_atlas(textures_to_use)
                self.master.after(0, lambda: messagebox.showinfo("Success", "Texture atlas created successfully!"))
            except Exception as e:
                self.master.after(0, lambda: messagebox.showerror("Error", f"Failed to create atlas: {e}"))
            finally:
                # Re-enable widgets
                self.master.after(0, self.enable_widgets)
                self.master.after(0, self.update_status)
        
        threading.Thread(target=create_thread, daemon=True).start()
    
    def disable_widget(self, widget):
        """Recursively disable widgets"""
        try:
            widget.config(state='disabled')
        except:
            pass
        for child in widget.winfo_children():
            self.disable_widget(child)
    
    def enable_widgets(self):
        """Re-enable all widgets"""
        for widget in self.master.winfo_children():
            self.enable_widget(widget)
    
    def enable_widget(self, widget):
        """Recursively enable widgets"""
        try:
            widget.config(state='normal')
        except:
            pass
        for child in widget.winfo_children():
            self.enable_widget(child)

def run_ui():
    """Run the texture atlas UI"""
    root = tk.Tk()
    TextureAtlasUI(root)
    root.mainloop()

if __name__ == "__main__":
    # Check if we should run the UI or command line version
    if len(sys.argv) > 1 and sys.argv[1] == "--no-ui":
        create_texture_atlas()
    else:
        run_ui()